use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use quartz_download::{DownloadItem, ParallelDownloader};
use serde_json::Value;
use thiserror::Error;
use zip::ZipArchive;

use crate::version::{
    is_native_artifact_path, library_allowed, library_artifact_path, native_classifier,
    resolve_version, ResolvedVersion,
};

#[derive(Debug, Clone, Copy)]
pub struct InstallOptions {

    pub download_assets: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            download_assets: true,
        }
    }
}

impl InstallOptions {
    pub fn for_launch() -> Self {
        Self {
            download_assets: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstalledVersion {
    pub id: String,
    pub main_class: String,
    pub classpath: Vec<PathBuf>,
    pub asset_index_id: String,
    pub game_dir: PathBuf,
    pub natives_dir: PathBuf,
}

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("version error: {0}")]
    Version(#[from] crate::version::VersionError),
    #[error("download error: {0}")]
    Download(#[from] quartz_download::DownloadError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

pub async fn install_version(
    version_url: &str,
    game_dir: impl AsRef<Path>,
) -> Result<InstalledVersion, InstallError> {
    install_version_with_options(version_url, game_dir, InstallOptions::default()).await
}

pub async fn install_version_with_options(
    version_url: &str,
    game_dir: impl AsRef<Path>,
    options: InstallOptions,
) -> Result<InstalledVersion, InstallError> {
    let game_dir = game_dir.as_ref();
    let resolved = resolve_version(version_url).await?;
    install_resolved(&resolved, game_dir, options).await
}

pub async fn install_version_by_id(
    version_id: &str,
    game_dir: impl AsRef<Path>,
) -> Result<InstalledVersion, InstallError> {
    install_version_by_id_with_options(version_id, game_dir, InstallOptions::default()).await
}

pub async fn install_version_by_id_with_options(
    version_id: &str,
    game_dir: impl AsRef<Path>,
    options: InstallOptions,
) -> Result<InstalledVersion, InstallError> {
    let game_dir = game_dir.as_ref();
    let resolved = crate::version::resolve_version_by_id(version_id).await?;
    install_resolved(&resolved, game_dir, options).await
}

async fn install_resolved(
    resolved: &ResolvedVersion,
    game_dir: &Path,
    options: InstallOptions,
) -> Result<InstalledVersion, InstallError> {
    tokio::fs::create_dir_all(game_dir).await?;

    let version_dir = game_dir
        .join("versions")
        .join(&resolved.id);
    tokio::fs::create_dir_all(&version_dir).await?;

    let client_jar = version_dir.join(format!("{}.jar", resolved.id));
    let mut downloads = Vec::new();
    if !client_jar.is_file() {
        downloads.push(DownloadItem {
            url: resolved.client_download.url.clone(),
            destination: client_jar.clone(),
            expected_sha1: Some(resolved.client_download.sha1.clone()),
        });
    }

    let mut classpath = vec![client_jar.clone()];
    let mut native_jars = Vec::new();

    for lib in &resolved.libraries {
        if !library_allowed(&lib.rules) {
            continue;
        }
        if let Some(ref downloads_meta) = lib.downloads {
            if let Some(ref artifact) = downloads_meta.artifact {
                let dest = library_artifact_path(game_dir, &artifact.path);
                if !dest.is_file() {
                    downloads.push(DownloadItem {
                        url: artifact.url.clone(),
                        destination: dest.clone(),
                        expected_sha1: Some(artifact.sha1.clone()),
                    });
                }
                if is_native_artifact_path(&artifact.path) {
                    native_jars.push(dest);
                } else {
                    classpath.push(dest);
                }
            }
            if let Some(ref classifiers) = downloads_meta.classifiers {
                let key = native_classifier();
                if let Some(native) = classifiers.get(key) {
                    if let Some(native) = crate::version::parse_artifact(native) {
                        let dest = library_artifact_path(game_dir, &native.path);
                        native_jars.push(dest.clone());
                        if !dest.is_file() {
                            downloads.push(DownloadItem {
                                url: native.url.clone(),
                                destination: dest,
                                expected_sha1: Some(native.sha1.clone()),
                            });
                        }
                    }
                }
            }
        }
    }

    let assets_dir = game_dir.join("assets");
    let indexes_dir = assets_dir.join("indexes");
    tokio::fs::create_dir_all(&indexes_dir).await?;
    let index_path = indexes_dir.join(format!("{}.json", resolved.asset_index.id));
    if !index_path.is_file() {
        downloads.push(DownloadItem {
            url: resolved.asset_index.url.clone(),
            destination: index_path.clone(),
            expected_sha1: Some(resolved.asset_index.sha1.clone()),
        });
    }

    if !downloads.is_empty() {
        ParallelDownloader::new(8)
            .download(downloads)
            .await?;
    }

    let natives_dir = version_dir.join("natives");
    if !native_jars.is_empty() {
        native_jars.sort();
        native_jars.dedup();
        let stamp = native_jars_stamp(&native_jars);
        let stamp_path = version_dir.join(".quartz-natives-stamp");
        let needs_extract = std::fs::read_to_string(&stamp_path)
            .map(|saved| saved.trim() != stamp)
            .unwrap_or(true);
        if needs_extract {
            if natives_dir.exists() {
                std::fs::remove_dir_all(&natives_dir)?;
            }
            extract_native_jars(&native_jars, &natives_dir)?;
            std::fs::write(&stamp_path, stamp)?;
        }
    }

    if options.download_assets {
        let index_raw = tokio::fs::read_to_string(&index_path).await?;
        let index: Value = serde_json::from_str(&index_raw)?;
        let mut asset_downloads = Vec::new();
        if let Some(objects) = index.get("objects").and_then(|o| o.as_object()) {
            for (_key, obj) in objects {
                let hash = obj["hash"].as_str().unwrap_or_default();
                if hash.is_empty() {
                    continue;
                }
                let prefix = &hash[..2];
                let dest = assets_dir.join("objects").join(prefix).join(hash);
                if dest.exists() {
                    continue;
                }
                let url = format!(
                    "https://resources.download.minecraft.net/{prefix}/{hash}"
                );
                asset_downloads.push(DownloadItem {
                    url,
                    destination: dest,
                    expected_sha1: Some(hash.to_owned()),
                });
            }
        }

        if !asset_downloads.is_empty() {
            ParallelDownloader::new(8)
                .download(asset_downloads)
                .await?;
        }
    }

    let version_json_path = version_dir.join(format!("{}.json", resolved.id));
    let version_json = serde_json::json!({
        "id": resolved.id,
        "mainClass": resolved.main_class,
        "assetIndex": resolved.asset_index.id,
    });
    tokio::fs::write(version_json_path, serde_json::to_string_pretty(&version_json)?).await?;

    Ok(InstalledVersion {
        id: resolved.id.clone(),
        main_class: resolved.main_class.clone(),
        classpath,
        asset_index_id: resolved.asset_index.id.clone(),
        game_dir: game_dir.to_path_buf(),
        natives_dir,
    })
}

fn native_jars_stamp(jars: &[PathBuf]) -> String {
    jars.iter()
        .filter_map(|jar| {
            let meta = std::fs::metadata(jar).ok()?;
            let modified = meta
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            Some(format!("{}:{}:{modified}", jar.display(), meta.len()))
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn extract_native_jars(jars: &[PathBuf], dest: &Path) -> Result<(), InstallError> {
    std::fs::create_dir_all(dest)?;
    for jar in jars {
        if !jar.is_file() {
            continue;
        }
        let file = File::open(jar)?;
        let mut archive = ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if entry.is_dir() {
                continue;
            }
            let name = entry.name().replace('/', std::path::MAIN_SEPARATOR_STR);
            if name.starts_with("META-INF") {
                continue;
            }
            let out_path = dest.join(&name);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = File::create(&out_path)?;
            io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}
