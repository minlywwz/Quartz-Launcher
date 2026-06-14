use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";

#[derive(Debug, Clone)]
pub struct CurseForgeClient {
    http: Client,
    api_key: String,
    base_url: String,
}

impl CurseForgeClient {
    pub fn from_env() -> Result<Self, CurseForgeError> {
        let api_key = std::env::var("CURSEFORGE_API_KEY").map_err(|_| {
            CurseForgeError::MissingApiKey(
                "CURSEFORGE_API_KEY environment variable is not set".into(),
            )
        })?;
        Ok(Self::new(api_key))
    }

    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            api_key: api_key.into(),
            base_url: CURSEFORGE_API_BASE.into(),
        }
    }

    pub async fn search_modpacks(
        &self,
        mc_version: &str,
        query: &str,
    ) -> Result<Vec<ModpackHit>, CurseForgeError> {
        let url = format!("{}/mods/search", self.base_url);
        let response = self
            .http
            .get(&url)
            .header("x-api-key", &self.api_key)
            .query(&[
                ("gameId", "432"),
                ("classId", "4471"),
                ("searchFilter", query),
                ("gameVersion", mc_version),
                ("pageSize", "20"),
                ("sortField", "2"),
                ("sortOrder", "desc"),
            ])
            .send()
            .await?
            .error_for_status()?;

        let body: CurseForgeSearchResponse = response.json().await?;
        Ok(body
            .data
            .into_iter()
            .map(|m| ModpackHit {
                id: m.id.to_string(),
                slug: m.slug,
                name: m.name,
                summary: m.summary,
                logo_url: m.logo.as_ref().map(|l| l.url.clone()),
                download_count: m.download_count,
            })
            .collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackHit {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub download_count: u64,
}

#[derive(Debug, Deserialize)]
struct CurseForgeSearchResponse {
    data: Vec<CurseForgeMod>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeMod {
    id: u64,
    slug: String,
    name: String,
    summary: String,
    download_count: u64,
    logo: Option<CurseForgeLogo>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeLogo {
    url: String,
}

#[derive(Debug, Error)]
pub enum CurseForgeError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("{0}")]
    MissingApiKey(String),
}
