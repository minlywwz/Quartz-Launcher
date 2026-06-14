use serde::{Deserialize, Serialize};
use thiserror::Error;

use quartz_meta::fetch_version_manifest;
use quartz_modplatforms::ModrinthClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModpackSource {
    Modrinth,
    Curseforge,
    Vanilla,
    Preset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub minecraft_version: String,
    pub source: ModpackSource,
    #[serde(default)]
    pub loader: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionCategory {
    pub id: String,
    pub label: String,
    pub is_latest: bool,
    pub modpacks: Vec<ModpackEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CatalogTree {
    pub categories: Vec<VersionCategory>,
}

impl CatalogTree {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct CatalogService {
    modrinth: ModrinthClient,
}

impl Default for CatalogService {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CatalogBuildOptions {
    pub include_snapshots: bool,
}

impl Default for CatalogBuildOptions {
    fn default() -> Self {
        Self {
            include_snapshots: false,
        }
    }
}

impl CatalogService {
    pub fn new() -> Self {
        Self {
            modrinth: ModrinthClient::default(),
        }
    }

    pub async fn build_tree(&self, options: &CatalogBuildOptions) -> Result<CatalogTree, CatalogError> {
        let manifest = fetch_version_manifest().await?;
        let latest_release = manifest.latest.release.clone();

        let filtered: Vec<_> = manifest
            .versions
            .iter()
            .filter(|v| {
                if options.include_snapshots && v.kind == "snapshot" {
                    return true;
                }
                if v.kind != "release" {
                    return false;
                }
                let pre = v.id.contains("-pre") || v.id.contains("-rc");
                if pre {
                    return options.include_snapshots;
                }
                v.id.starts_with("1.")
            })
            .collect();

        let mut categories = Vec::new();

        for (idx, entry) in filtered.iter().enumerate() {
            let is_latest = entry.id == latest_release;
            let mut modpacks = Vec::new();

            modpacks.push(ModpackEntry {
                id: format!("vanilla:{}", entry.id),
                name: format!("Minecraft {}", entry.id),
                version: entry.id.clone(),
                minecraft_version: entry.id.clone(),
                source: ModpackSource::Vanilla,
                loader: None,
                icon_url: None,
                slug: Some(entry.id.clone()),
                description: Some("Official vanilla Minecraft".into()),
            });

            if idx < 10 {
                match self.modrinth.search_modpacks(&entry.id, "", 6).await {
                    Ok(results) => {
                        for hit in results.hits {
                            modpacks.push(ModpackEntry {
                                id: format!("modrinth:{}", hit.slug),
                                name: hit.title,
                                version: hit
                                    .versions
                                    .first()
                                    .cloned()
                                    .unwrap_or_else(|| entry.id.clone()),
                                minecraft_version: entry.id.clone(),
                                source: ModpackSource::Modrinth,
                                loader: None,
                                icon_url: hit.icon_url,
                                slug: Some(hit.slug),
                                description: Some(hit.description),
                            });
                        }
                    }
                    Err(_) => {}
                }
            }

            categories.push(VersionCategory {
                id: entry.id.clone(),
                label: entry.id.clone(),
                is_latest,
                modpacks,
            });
        }

        Ok(CatalogTree { categories })
    }

    pub async fn search_modpacks(
        &self,
        mc_version: &str,
        query: &str,
        limit: u32,
    ) -> Result<Vec<ModpackEntry>, CatalogError> {
        let results = self
            .modrinth
            .search_modpacks(mc_version, query, limit)
            .await?;

        Ok(results
            .hits
            .into_iter()
            .map(|hit| ModpackEntry {
                id: format!("modrinth:{}", hit.slug),
                name: hit.title,
                version: hit
                    .versions
                    .first()
                    .cloned()
                    .unwrap_or_else(|| mc_version.to_owned()),
                minecraft_version: mc_version.to_owned(),
                source: ModpackSource::Modrinth,
                loader: None,
                icon_url: hit.icon_url,
                slug: Some(hit.slug),
                description: Some(hit.description),
            })
            .collect())
    }
}

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("manifest error: {0}")]
    Manifest(#[from] quartz_meta::ManifestError),
    #[error("modrinth error: {0}")]
    Modrinth(#[from] quartz_modplatforms::ModrinthError),
    #[error("failed to parse catalog JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl CatalogTree {
    pub fn from_json(json: &str) -> Result<Self, CatalogError> {
        Ok(serde_json::from_str(json)?)
    }

    pub fn to_json(&self) -> Result<String, CatalogError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}
