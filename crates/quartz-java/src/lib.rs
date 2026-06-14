use std::path::{Path, PathBuf};

use quartz_download::{DownloadItem, ParallelDownloader};
use quartz_meta::{resolve_version_by_id, JavaVersionRef, VersionError};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use sha1::{Digest, Sha1};
use thiserror::Error;

const RUNTIME_INDEX_URL: &str =
    "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Debug, Error)]
pub enum JavaError {
    #[error("version error: {0}")]
    Version(#[from] VersionError),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("download error: {0}")]
    Download(#[from] quartz_download::DownloadError),
    #[error("no java executable found in runtime")]
    MissingExecutable,
    #[error("runtime manifest missing for component {0}")]
    MissingManifest(String),
}

pub async fn ensure_java_for_version(
    version_id: &str,
    data_dir: &Path,
) -> Result<PathBuf, JavaError> {
    let resolved = resolve_version_by_id(version_id).await?;
    let java_ref = resolved
        .java_version
        .clone()
        .unwrap_or_else(|| fallback_java_ref(version_id));

    install_runtime(&java_ref.component, data_dir).await
}

async fn resolve_runtime_manifest(component: &str) -> Result<(String, String), JavaError> {
    let platform = runtime_platform();
    let index: Value = Client::new()
        .get(RUNTIME_INDEX_URL)
        .send()
        .await?
        .json()
        .await?;

    let entry = index
        .get(platform)
        .and_then(|p| p.get(component))
        .and_then(|c| c.as_array())
        .and_then(|entries| entries.first())
        .ok_or_else(|| {
            JavaError::MissingManifest(format!("{component} on {platform}"))
        })?;

    let manifest = entry
        .get("manifest")
        .ok_or_else(|| JavaError::MissingManifest(component.to_owned()))?;

    let url = manifest
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JavaError::MissingManifest(component.to_owned()))?
        .to_owned();
    let sha1 = manifest
        .get("sha1")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JavaError::MissingManifest(component.to_owned()))?
        .to_owned();

    Ok((url, sha1))
}

pub async fn install_runtime(component: &str, data_dir: &Path) -> Result<PathBuf, JavaError> {
    let (manifest_url, manifest_sha1) = resolve_runtime_manifest(component).await?;

    let raw: Value = Client::new()
        .get(&manifest_url)
        .send()
        .await?
        .json()
        .await?;

    let runtime_root = data_dir
        .join("runtime")
        .join(component)
        .join(&manifest_sha1);

    if let Ok(java_exe) = find_java_executable(&runtime_root) {
        return Ok(java_exe);
    }

    let files = raw
        .get("files")
        .and_then(|v| v.as_object())
        .ok_or_else(|| JavaError::MissingManifest(component.to_owned()))?;

    let mut items = Vec::new();
    for (rel_path, entry) in files {
        let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("file");
        let dest = runtime_root.join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));

        if entry_type == "directory" {
            tokio::fs::create_dir_all(&dest).await?;
            continue;
        }

        if let Some(downloads) = entry.get("downloads").and_then(|d| d.get("raw")) {
            let url = downloads
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned();
            let sha1 = downloads
                .get("sha1")
                .and_then(|v| v.as_str())
                .map(str::to_owned);

            if !url.is_empty() {
                if dest.is_file() {
                    continue;
                }
                if let Some(parent) = dest.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                items.push(DownloadItem {
                    url,
                    destination: dest.clone(),
                    expected_sha1: sha1,
                });
            }
        }

        if entry.get("executable").and_then(|v| v.as_bool()) == Some(true) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if dest.exists() {
                    let mut perms = tokio::fs::metadata(&dest).await?.permissions();
                    perms.set_mode(0o755);
                    tokio::fs::set_permissions(&dest, perms).await?;
                }
            }
        }
    }

    if !items.is_empty() {
        ParallelDownloader::new(8).download(items).await?;
    }

    find_java_executable(&runtime_root)
}

fn find_java_executable(runtime_root: &Path) -> Result<PathBuf, JavaError> {
    let candidates = if cfg!(windows) {
        vec![
            runtime_root.join("bin").join("java.exe"),
            runtime_root.join("bin").join("javaw.exe"),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            runtime_root.join("bin").join("java"),
            runtime_root.join("jre.bundle").join("Contents").join("Home").join("bin").join("java"),
        ]
    } else {
        vec![runtime_root.join("bin").join("java")]
    };

    for path in candidates {
        if path.is_file() {
            return Ok(path);
        }
    }

    Err(JavaError::MissingExecutable)
}

fn runtime_platform() -> &'static str {
    if cfg!(windows) {
        if cfg!(target_arch = "x86_64") {
            "windows-x64"
        } else {
            "windows-x86"
        }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "mac-os-arm64"
        } else {
            "mac-os"
        }
    } else if cfg!(target_arch = "x86_64") {
        "linux-x64"
    } else {
        "linux-i386"
    }
}

fn fallback_java_ref(version_id: &str) -> JavaVersionRef {
    let major = parse_mc_major(version_id);
    if major >= 21 || (major == 20 && parse_mc_minor(version_id) >= 5) {
        JavaVersionRef {
            component: "java-runtime-delta".into(),
            major_version: 21,
        }
    } else if major >= 17 {
        JavaVersionRef {
            component: "java-runtime-gamma".into(),
            major_version: 17,
        }
    } else if major >= 16 {
        JavaVersionRef {
            component: "java-runtime-beta".into(),
            major_version: 16,
        }
    } else {
        JavaVersionRef {
            component: "jre-legacy".into(),
            major_version: 8,
        }
    }
}

fn parse_mc_major(version_id: &str) -> u32 {
    version_id
        .split('.')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn parse_mc_minor(version_id: &str) -> u32 {
    version_id
        .split('.')
        .nth(2)
        .and_then(|s| s.split('-').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

#[derive(Debug, Deserialize)]
struct RuntimeManifestHead {
    manifest: RuntimeManifestMeta,
}

#[derive(Debug, Deserialize)]
struct RuntimeManifestMeta {
    sha1: String,
}

#[allow(dead_code)]
fn manifest_sha1(raw: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}
