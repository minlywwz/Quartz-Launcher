use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod manifest;
pub mod fabric;
pub mod version;
pub mod install;
pub mod loaders;
pub mod launch_profile;

pub use manifest::{fetch_version_manifest, VersionManifest, VersionManifestEntry, ManifestError};
pub use fabric::{fetch_profile, latest_loader, FabricError};
pub use install::{
    install_version, install_version_by_id, install_version_by_id_with_options, InstalledVersion,
    InstallError, InstallOptions,
};
pub use loaders::{
    ensure_loader_for_launch, install_with_loader, LoaderKind, LoaderInstallResult, LoaderError,
};
pub use launch_profile::{LaunchProfile, LaunchProfileError};
pub use version::{
    resolve_version, resolve_version_by_id, JavaVersionRef, ResolvedVersion, VersionError,
};

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionJson {
    pub id: String,
    #[serde(default)]
    pub main_class: Option<String>,
    pub downloads: VersionDownloads,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: DownloadEntry,
    #[serde(default)]
    pub server: Option<DownloadEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadEntry {
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Error)]
pub enum VersionJsonError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("failed to parse version JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("install error: {0}")]
    Install(String),
}

pub async fn fetch_version_json(url: &str) -> Result<VersionJson, VersionJsonError> {
    let version = reqwest::get(url).await?.json().await?;
    Ok(version)
}

pub async fn install_vanilla(
    version_url: &str,
    game_dir: impl AsRef<Path>,
) -> Result<PathBuf, VersionJsonError> {
    let installed = install_version(version_url, game_dir)
        .await
        .map_err(|e| VersionJsonError::Install(e.to_string()))?;
    Ok(installed
        .classpath
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("client.jar")))
}
