use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone)]
pub struct ModrinthClient {
    http: Client,
    base_url: String,
}

impl Default for ModrinthClient {
    fn default() -> Self {
        Self::new(MODRINTH_API_BASE)
    }
}

impl ModrinthClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.into(),
        }
    }

    pub async fn search_modpacks(
        &self,
        mc_version: &str,
        query: &str,
        limit: u32,
    ) -> Result<ModrinthSearchResponse, ModrinthError> {
        let facets = format!(
            r#"[["project_type:modpack"],["versions:{}"]]"#,
            mc_version
        );
        let url = format!(
            "{}/search?query={}&facets={}&limit={}",
            self.base_url,
            urlencoding::encode(query),
            urlencoding::encode(&facets),
            limit
        );
        let response = self.http.get(&url).send().await?.json().await?;
        Ok(response)
    }

    pub async fn get_game_versions(&self) -> Result<Vec<ModrinthGameVersion>, ModrinthError> {
        let url = format!("{}/tag/game_version", self.base_url);
        let versions = self.http.get(&url).send().await?.json().await?;
        Ok(versions)
    }

    pub async fn get_project_versions(
        &self,
        slug: &str,
    ) -> Result<Vec<ModrinthVersion>, ModrinthError> {
        let url = format!(
            "{}/project/{}/version",
            self.base_url,
            urlencoding::encode(slug)
        );
        let versions = self.http.get(&url).send().await?.json().await?;
        Ok(versions)
    }

    pub async fn get_version_download_url(
        &self,
        version_id: &str,
    ) -> Result<String, ModrinthError> {
        let url = format!(
            "{}/version/{}",
            self.base_url,
            urlencoding::encode(version_id)
        );
        let version: ModrinthVersion = self.http.get(&url).send().await?.json().await?;
        version
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| version.files.first())
            .map(|f| f.url.clone())
            .ok_or_else(|| ModrinthError::NoDownloadFile(version_id.to_owned()))
    }

    pub async fn search(&self, query: &str, limit: u32) -> Result<ModrinthSearchResponse, ModrinthError> {
        let url = format!(
            "{}/search?query={}&limit={}",
            self.base_url,
            urlencoding::encode(query),
            limit
        );
        let response = self.http.get(&url).send().await?.json().await?;
        Ok(response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthSearchResponse {
    pub hits: Vec<ModrinthHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthGameVersion {
    pub version: String,
    #[serde(default)]
    pub version_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    pub files: Vec<ModrinthVersionFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthVersionFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub hashes: ModrinthFileHashes,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModrinthFileHashes {
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
}

#[derive(Debug, Error)]
pub enum ModrinthError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("no download file for version {0}")]
    NoDownloadFile(String),
}
