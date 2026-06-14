use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub time: String,
    #[serde(default)]
    pub sha1: Option<String>,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("failed to parse manifest JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub async fn fetch_version_manifest() -> Result<VersionManifest, ManifestError> {
    let manifest = reqwest::get(VERSION_MANIFEST_URL).await?.json().await?;
    Ok(manifest)
}
