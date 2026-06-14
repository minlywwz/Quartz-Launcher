use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::manifest::{fetch_version_manifest, ManifestError};

#[derive(Debug, Clone)]
pub struct JavaVersionRef {
    pub component: String,
    pub major_version: u32,
}

#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    pub id: String,
    pub main_class: String,
    pub libraries: Vec<Library>,
    pub asset_index: AssetIndexRef,
    pub client_download: ClientDownload,
    pub java_version: Option<JavaVersionRef>,
}

#[derive(Debug, Clone)]
pub struct ClientDownload {
    pub url: String,
    pub sha1: String,
}

#[derive(Debug, Clone)]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    pub sha1: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<LibraryRule>,
    #[serde(default)]
    pub natives: Option<serde_json::Map<String, Value>>,
    #[serde(default)]
    pub extract: Option<LibraryExtract>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<ArtifactDownload>,
    #[serde(default)]
    pub classifiers: Option<serde_json::Map<String, Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArtifactDownload {
    pub path: String,
    pub url: String,
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryRule {
    pub action: String,
    #[serde(default)]
    pub os: Option<LibraryRuleOs>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryRuleOs {
    pub name: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryExtract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("version {0} not found in manifest")]
    NotFound(String),
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),
}

pub async fn resolve_version(version_url: &str) -> Result<ResolvedVersion, VersionError> {
    let mut chain = Vec::new();
    let mut url = version_url.to_owned();

    loop {
        let raw: Value = reqwest::get(&url).await?.json().await?;
        let inherits = raw
            .get("inheritsFrom")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        chain.push(raw);
        if let Some(parent_id) = inherits {
            let manifest = fetch_version_manifest().await?;
            let parent = manifest
                .versions
                .iter()
                .find(|v| v.id == parent_id)
                .ok_or_else(|| VersionError::NotFound(parent_id.clone()))?;
            url = parent.url.clone();
        } else {
            break;
        }
    }

    chain.reverse();
    merge_chain(&chain)
}

pub async fn resolve_version_by_id(version_id: &str) -> Result<ResolvedVersion, VersionError> {
    let manifest = fetch_version_manifest().await?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| VersionError::NotFound(version_id.to_owned()))?;
    resolve_version(&entry.url).await
}

fn merge_chain(chain: &[Value]) -> Result<ResolvedVersion, VersionError> {
    let mut libraries = Vec::new();
    let mut id = String::new();
    let mut main_class = String::new();
    let mut client_download = None;
    let mut asset_index = None;
    let mut java_version = None;

    for node in chain {
        if let Some(v) = node.get("id").and_then(|v| v.as_str()) {
            id = v.to_owned();
        }
        if let Some(v) = node.get("mainClass").and_then(|v| v.as_str()) {
            main_class = v.to_owned();
        }
        if let Some(downloads) = node.get("downloads") {
            if let Some(client) = downloads.get("client") {
                client_download = Some(ClientDownload {
                    url: client["url"].as_str().unwrap_or_default().to_owned(),
                    sha1: client["sha1"].as_str().unwrap_or_default().to_owned(),
                });
            }
        }
        if let Some(idx) = node.get("assetIndex") {
            asset_index = Some(AssetIndexRef {
                id: idx["id"].as_str().unwrap_or_default().to_owned(),
                url: idx["url"].as_str().unwrap_or_default().to_owned(),
                sha1: idx["sha1"].as_str().unwrap_or_default().to_owned(),
            });
        }
        if let Some(arr) = node.get("libraries").and_then(|v| v.as_array()) {
            for lib in arr {
                let lib: Library = serde_json::from_value(lib.clone())?;
                libraries.push(lib);
            }
        }
        if let Some(jv) = node.get("javaVersion") {
            if let (Some(component), Some(major)) = (
                jv.get("component").and_then(|v| v.as_str()),
                jv.get("majorVersion").and_then(|v| v.as_u64()),
            ) {
                java_version = Some(JavaVersionRef {
                    component: component.to_owned(),
                    major_version: major as u32,
                });
            }
        }
    }

    Ok(ResolvedVersion {
        id,
        main_class,
        libraries,
        asset_index: asset_index.ok_or(VersionError::MissingField("assetIndex"))?,
        client_download: client_download.ok_or(VersionError::MissingField("downloads.client"))?,
        java_version,
    })
}

pub fn os_arch() -> &'static str {
    if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    }
}

pub fn library_allowed(rules: &[LibraryRule]) -> bool {
    if rules.is_empty() {
        return true;
    }
    let os = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };
    let arch = os_arch();

    let mut allowed = false;
    for rule in rules {
        let os_match = rule
            .os
            .as_ref()
            .and_then(|o| o.name.as_deref())
            .map(|name| name == os)
            .unwrap_or(true);
        let arch_match = rule
            .os
            .as_ref()
            .and_then(|o| o.arch.as_deref())
            .map(|expected| expected == arch)
            .unwrap_or(true);
        if os_match && arch_match {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

pub fn is_native_artifact_path(path: &str) -> bool {
    path.contains("-natives-")
}

pub fn library_artifact_path(game_dir: &Path, artifact_path: &str) -> PathBuf {
    game_dir.join("libraries").join(artifact_path)
}

pub fn native_classifier() -> &'static str {
    if cfg!(windows) {
        if cfg!(target_arch = "aarch64") {
            "natives-windows-arm64"
        } else if cfg!(target_arch = "x86") {
            "natives-windows-x86"
        } else {
            "natives-windows"
        }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "natives-macos-arm64"
        } else {
            "natives-macos"
        }
    } else if cfg!(target_arch = "aarch64") {
        "natives-linux-arm64"
    } else if cfg!(target_arch = "arm") {
        "natives-linux-arm32"
    } else {
        "natives-linux"
    }
}

pub fn parse_artifact(value: &Value) -> Option<ArtifactDownload> {
    serde_json::from_value(value.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(action: &str, os_name: Option<&str>, arch: Option<&str>) -> LibraryRule {
        LibraryRule {
            action: action.to_owned(),
            os: Some(LibraryRuleOs {
                name: os_name.map(str::to_owned),
                arch: arch.map(str::to_owned),
            }),
        }
    }

    #[test]
    fn library_allowed_matches_arch_when_specified() {
        let rules = vec![rule("allow", Some("windows"), Some("x64"))];
        if cfg!(windows) && cfg!(target_arch = "x86_64") {
            assert!(library_allowed(&rules));
        }
        let arm_rules = vec![rule("allow", Some("windows"), Some("arm64"))];
        if cfg!(windows) && cfg!(target_arch = "aarch64") {
            assert!(library_allowed(&arm_rules));
        }
        if cfg!(windows) && cfg!(target_arch = "x86_64") {
            assert!(!library_allowed(&arm_rules));
        }
    }

    #[test]
    fn is_native_artifact_path_detects_natives_jars() {
        assert!(is_native_artifact_path(
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar"
        ));
        assert!(!is_native_artifact_path(
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar"
        ));
    }

    #[test]
    fn native_classifier_windows_x64() {
        if cfg!(windows) && cfg!(target_arch = "x86_64") {
            assert_eq!(native_classifier(), "natives-windows");
        }
    }
}
