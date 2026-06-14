use std::path::{Path, PathBuf};
use std::process::Stdio;

use quartz_download::{DownloadItem, ParallelDownloader};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use tokio::process::Command;

use crate::fabric::{fetch_profile, latest_loader, FabricError};
use crate::install::{
    install_version_by_id, install_version_by_id_with_options, InstalledVersion, InstallError,
    InstallOptions,
};
use crate::launch_profile::jvm_args_from_profile;
use crate::version::{is_native_artifact_path, library_allowed, library_artifact_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderKind {
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl LoaderKind {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fabric" => Self::Fabric,
            "quilt" => Self::Quilt,
            "forge" => Self::Forge,
            "neoforge" => Self::NeoForge,
            _ => Self::Vanilla,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoaderInstallResult {
    pub loader_version: String,
    pub main_class: String,
    pub classpath: Vec<PathBuf>,
    pub version_id: String,
    pub jvm_args: Vec<String>,
}

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("install error: {0}")]
    Install(#[from] InstallError),
    #[error("fabric error: {0}")]
    Fabric(#[from] FabricError),
    #[error("download error: {0}")]
    Download(#[from] quartz_download::DownloadError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("loader install failed: {0}")]
    Process(String),
    #[error("no loader version found")]
    NoVersion,
}

pub async fn ensure_loader_for_launch(
    mc_version: &str,
    loader: LoaderKind,
    game_dir: &Path,
    java: &str,
) -> Result<(InstalledVersion, LoaderInstallResult), LoaderError> {
    let options = InstallOptions::for_launch();
    let base = install_version_by_id_with_options(mc_version, game_dir, options).await?;

    let loader_result = match loader {
        LoaderKind::Vanilla => loader_result_from_base(&base),
        LoaderKind::Fabric => {
            if let Some(cached) = load_cached_loader_profile(game_dir, mc_version, "fabric", &base).await?
            {
                cached
            } else {
                install_fabric_profile(mc_version, game_dir, &base).await?
            }
        }
        LoaderKind::Quilt => {
            if let Some(cached) = load_cached_loader_profile(game_dir, mc_version, "quilt", &base).await?
            {
                cached
            } else {
                install_quilt_profile(mc_version, game_dir, &base).await?
            }
        }
        LoaderKind::Forge | LoaderKind::NeoForge => {

            return match loader {
                LoaderKind::Forge => {
                    let result = install_forge(mc_version, game_dir, java, &base).await?;
                    Ok((base, result))
                }
                LoaderKind::NeoForge => {
                    let result = install_neoforge(mc_version, game_dir, java, &base).await?;
                    Ok((base, result))
                }
                _ => unreachable!(),
            };
        }
    };

    Ok((base, loader_result))
}

fn loader_result_from_base(base: &InstalledVersion) -> LoaderInstallResult {
    LoaderInstallResult {
        loader_version: String::new(),
        main_class: base.main_class.clone(),
        classpath: base.classpath.clone(),
        version_id: base.id.clone(),
        jvm_args: Vec::new(),
    }
}

async fn load_cached_loader_profile(
    game_dir: &Path,
    mc_version: &str,
    loader_name: &str,
    base: &InstalledVersion,
) -> Result<Option<LoaderInstallResult>, LoaderError> {
    let prefix = format!("{loader_name}-loader-{mc_version}-");
    let versions_dir = game_dir.join("versions");
    let Ok(mut entries) = tokio::fs::read_dir(&versions_dir).await else {
        return Ok(None);
    };

    let mut version_id = None;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&prefix) {
            version_id = Some(name);
            break;
        }
    }

    let Some(version_id) = version_id else {
        return Ok(None);
    };

    let json_path = versions_dir.join(&version_id).join(format!("{version_id}.json"));
    if !json_path.is_file() {
        return Ok(None);
    }

    let profile: Value = serde_json::from_str(&tokio::fs::read_to_string(&json_path).await?)?;
    let loader_ver = version_id
        .strip_prefix(&prefix)
        .unwrap_or("unknown")
        .to_owned();

    let main_class = profile["mainClass"]
        .as_str()
        .unwrap_or(&base.main_class)
        .to_owned();
    let classpath = classpath_from_profile_libraries(game_dir, &profile, base)?;

    Ok(Some(LoaderInstallResult {
        loader_version: loader_ver,
        main_class,
        classpath,
        version_id,
        jvm_args: jvm_args_from_profile(&profile),
    }))
}

fn classpath_from_profile_libraries(
    game_dir: &Path,
    profile: &Value,
    base: &InstalledVersion,
) -> Result<Vec<PathBuf>, LoaderError> {
    let mut classpath = base.classpath.clone();

    if let Some(libraries) = profile.get("libraries").and_then(|v| v.as_array()) {
        for lib_value in libraries {
            let lib: crate::version::Library = serde_json::from_value(lib_value.clone())?;
            if !library_allowed(&lib.rules) {
                continue;
            }
            if let Some(ref downloads_meta) = lib.downloads {
                if let Some(ref artifact) = downloads_meta.artifact {
                    if !is_native_artifact_path(&artifact.path) {
                        classpath.push(library_artifact_path(game_dir, &artifact.path));
                    }
                }
            } else if let Some(rel_path) = maven_relative_path(&lib.name) {
                if !is_native_artifact_path(&rel_path) {
                    classpath.push(library_artifact_path(game_dir, &rel_path));
                }
            }
        }
    }

    Ok(classpath)
}

pub async fn install_with_loader(
    mc_version: &str,
    loader: LoaderKind,
    game_dir: &Path,
    java: &str,
) -> Result<LoaderInstallResult, LoaderError> {
    let base = install_version_by_id(mc_version, game_dir).await?;

    match loader {
        LoaderKind::Vanilla => Ok(LoaderInstallResult {
            loader_version: String::new(),
            main_class: base.main_class,
            classpath: base.classpath,
            version_id: base.id,
            jvm_args: Vec::new(),
        }),
        LoaderKind::Fabric => install_fabric_profile(mc_version, game_dir, &base).await,
        LoaderKind::Quilt => install_quilt_profile(mc_version, game_dir, &base).await,
        LoaderKind::Forge => install_forge(mc_version, game_dir, java, &base).await,
        LoaderKind::NeoForge => install_neoforge(mc_version, game_dir, java, &base).await,
    }
}

async fn install_fabric_profile(
    mc_version: &str,
    game_dir: &Path,
    base: &crate::install::InstalledVersion,
) -> Result<LoaderInstallResult, LoaderError> {
    let loader_ver = latest_loader(mc_version).await?;
    let profile = fetch_profile(mc_version, &loader_ver).await?;
    merge_loader_profile(game_dir, mc_version, &loader_ver, "fabric", profile, base).await
}

async fn install_quilt_profile(
    mc_version: &str,
    game_dir: &Path,
    base: &crate::install::InstalledVersion,
) -> Result<LoaderInstallResult, LoaderError> {
    let url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{mc_version}/latest/profile/json"
    );
    let profile: Value = reqwest::get(&url).await?.json().await?;
    let loader_ver = profile["loader"]["version"]
        .as_str()
        .unwrap_or("unknown")
        .to_owned();
    merge_loader_profile(game_dir, mc_version, &loader_ver, "quilt", profile, base).await
}

async fn merge_loader_profile(
    game_dir: &Path,
    mc_version: &str,
    loader_ver: &str,
    loader_name: &str,
    profile: Value,
    base: &crate::install::InstalledVersion,
) -> Result<LoaderInstallResult, LoaderError> {
    let version_id = format!("{loader_name}-loader-{mc_version}-{loader_ver}");
    let version_dir = game_dir.join("versions").join(&version_id);
    tokio::fs::create_dir_all(&version_dir).await?;

    let main_class = profile["mainClass"]
        .as_str()
        .unwrap_or(&base.main_class)
        .to_owned();

    let mut downloads = Vec::new();
    let mut classpath = base.classpath.clone();

    if let Some(libraries) = profile.get("libraries").and_then(|v| v.as_array()) {
        for lib_value in libraries {
            let lib: crate::version::Library = serde_json::from_value(lib_value.clone())?;
            if !library_allowed(&lib.rules) {
                continue;
            }
            if let Some(ref downloads_meta) = lib.downloads {
                if let Some(ref artifact) = downloads_meta.artifact {
                    if is_native_artifact_path(&artifact.path) {
                        continue;
                    }
                    let dest = library_artifact_path(game_dir, &artifact.path);
                    if !dest.exists() {
                        downloads.push(DownloadItem {
                            url: artifact.url.clone(),
                            destination: dest.clone(),
                            expected_sha1: Some(artifact.sha1.clone()),
                        });
                    }
                    classpath.push(dest);
                }
            } else if let Some(rel_path) = maven_relative_path(&lib.name) {
                if is_native_artifact_path(&rel_path) {
                    continue;
                }
                let base_url = lib_value
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("https://libraries.minecraft.net/");
                let url = format!("{}/{}", base_url.trim_end_matches('/'), rel_path);
                let dest = library_artifact_path(game_dir, &rel_path);
                if !dest.exists() {
                    downloads.push(DownloadItem {
                        url,
                        destination: dest.clone(),
                        expected_sha1: lib_value
                            .get("sha1")
                            .and_then(|v| v.as_str())
                            .map(str::to_owned),
                    });
                }
                classpath.push(dest);
            }
        }
    }

    if !downloads.is_empty() {
        ParallelDownloader::new(8).download(downloads).await?;
    }

    tokio::fs::write(
        version_dir.join(format!("{version_id}.json")),
        serde_json::to_string_pretty(&profile)?,
    )
    .await?;

    Ok(LoaderInstallResult {
        loader_version: loader_ver.to_owned(),
        main_class,
        classpath,
        version_id,
        jvm_args: jvm_args_from_profile(&profile),
    })
}

async fn install_forge(
    mc_version: &str,
    game_dir: &Path,
    java: &str,
    base: &crate::install::InstalledVersion,
) -> Result<LoaderInstallResult, LoaderError> {
    let forge_version = fetch_forge_version(mc_version).await?;
    let installer_url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{mc_version}-{forge_version}/forge-{mc_version}-{forge_version}-installer.jar"
    );
    run_loader_installer(
        game_dir,
        java,
        &installer_url,
        &format!("forge-{mc_version}-{forge_version}-installer.jar"),
        base,
        mc_version,
    )
    .await
}

async fn install_neoforge(
    mc_version: &str,
    game_dir: &Path,
    java: &str,
    base: &crate::install::InstalledVersion,
) -> Result<LoaderInstallResult, LoaderError> {
    let neoforge_version = fetch_neoforge_version(mc_version).await?;
    let installer_url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/neoforge-{neoforge_version}-installer.jar"
    );
    run_loader_installer(
        game_dir,
        java,
        &installer_url,
        &format!("neoforge-{neoforge_version}-installer.jar"),
        base,
        mc_version,
    )
    .await
}

async fn run_loader_installer(
    game_dir: &Path,
    java: &str,
    installer_url: &str,
    filename: &str,
    base: &crate::install::InstalledVersion,
    mc_version: &str,
) -> Result<LoaderInstallResult, LoaderError> {
    let installers_dir = game_dir.join("installers");
    tokio::fs::create_dir_all(&installers_dir).await?;
    let installer_path = installers_dir.join(filename);

    ParallelDownloader::new(1)
        .download(vec![DownloadItem {
            url: installer_url.to_owned(),
            destination: installer_path.clone(),
            expected_sha1: None,
        }])
        .await?;

    let output = Command::new(java)
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installClient")
        .current_dir(game_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LoaderError::Process(format!(
            "installer exited with {}: {stderr}",
            output.status
        )));
    }

    let versions_dir = game_dir.join("versions");
    let mut merged_classpath = base.classpath.clone();
    let mut version_id = mc_version.to_owned();
    let mut main_class = base.main_class.clone();

    if let Ok(mut entries) = tokio::fs::read_dir(&versions_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains("forge") || name.contains("neoforge") {
                version_id = name.clone();
                if let Ok(json) =
                    tokio::fs::read_to_string(entry.path().join(format!("{name}.json"))).await
                {
                    if let Ok(profile) = serde_json::from_str::<Value>(&json) {
                        main_class = profile["mainClass"]
                            .as_str()
                            .unwrap_or(&main_class)
                            .to_owned();
                        if let Some(libraries) = profile.get("libraries").and_then(|v| v.as_array()) {
                            for lib in libraries {
                                if let Ok(lib) = serde_json::from_value::<crate::version::Library>(lib.clone()) {
                                    if library_allowed(&lib.rules) {
                                        if let Some(artifact) = lib.downloads.and_then(|d| d.artifact) {
                                            if !is_native_artifact_path(&artifact.path) {
                                                merged_classpath
                                                    .push(library_artifact_path(game_dir, &artifact.path));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                break;
            }
        }
    }

    Ok(LoaderInstallResult {
        loader_version: version_id.clone(),
        main_class,
        classpath: merged_classpath,
        version_id,
        jvm_args: Vec::new(),
    })
}

async fn fetch_forge_version(mc_version: &str) -> Result<String, LoaderError> {
    let url = format!(
        "https://files.minecraftforge.net/net/minecraftforge/forge/index_{mc_version}.json"
    );
    let body: ForgeIndex = reqwest::get(&url).await?.json().await?;
    body.promos
        .get("recommended")
        .or_else(|| body.promos.get("latest"))
        .cloned()
        .ok_or(LoaderError::NoVersion)
}

async fn fetch_neoforge_version(mc_version: &str) -> Result<String, LoaderError> {
    let url = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
    let versions: Vec<String> = reqwest::get(url).await?.json().await?;
    versions
        .into_iter()
        .filter(|v| v.starts_with(mc_version))
        .next_back()
        .ok_or(LoaderError::NoVersion)
}

#[derive(Debug, Deserialize)]
struct ForgeIndex {
    promos: std::collections::HashMap<String, String>,
}

fn maven_relative_path(name: &str) -> Option<String> {
    let mut parts = name.split(':');
    let group = parts.next()?;
    let artifact = parts.next()?;
    let version = parts.next()?;
    let group_path = group.replace('.', "/");
    Some(format!(
        "{group_path}/{artifact}/{version}/{artifact}-{version}.jar"
    ))
}
