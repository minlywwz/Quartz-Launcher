

use serde::Deserialize;
use thiserror::Error;

pub const FABRIC_META_BASE: &str = "https://meta.fabricmc.net/v2";

#[derive(Debug, Error)]
pub enum FabricError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no fabric loader found for this Minecraft version")]
    NoLoader,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderVersion {
    version: String,
    stable: bool,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderVersion,
}

pub async fn latest_loader(mc_version: &str) -> Result<String, FabricError> {
    let url = format!("{FABRIC_META_BASE}/versions/loader/{mc_version}");
    let versions: Vec<FabricLoaderEntry> = reqwest::get(&url).await?.json().await?;
    versions
        .iter()
        .find(|v| v.loader.stable)
        .or_else(|| versions.first())
        .map(|v| v.loader.version.clone())
        .ok_or(FabricError::NoLoader)
}

pub fn profile_url(mc_version: &str, loader: &str) -> String {
    format!("{FABRIC_META_BASE}/versions/loader/{mc_version}/{loader}/profile/json")
}

pub async fn fetch_profile(mc_version: &str, loader: &str) -> Result<serde_json::Value, FabricError> {
    let url = profile_url(mc_version, loader);
    Ok(reqwest::get(&url).await?.json().await?)
}
