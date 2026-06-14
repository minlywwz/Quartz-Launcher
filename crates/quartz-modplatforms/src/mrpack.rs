use std::fs::File;
use std::io::{copy, Read};
use std::path::Path;

use quartz_download::{DownloadItem, ParallelDownloader};
use serde::Deserialize;
use thiserror::Error;
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub struct MrpackInstallResult {
    pub mods_installed: usize,
    pub overrides_copied: usize,
    pub game_version: String,
    pub loaders: Vec<String>,
}

#[derive(Debug, Error)]
pub enum MrpackError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("download error: {0}")]
    Download(#[from] quartz_download::DownloadError),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("invalid mrpack: {0}")]
    Invalid(&'static str),
}

#[derive(Debug, Deserialize)]
struct MrpackIndex {
    #[serde(default)]
    game: Option<String>,
    #[serde(default)]
    format_version: u32,
    #[serde(default)]
    dependencies: std::collections::HashMap<String, String>,
    files: Vec<MrpackFile>,
}

#[derive(Debug, Deserialize)]
struct MrpackFile {
    path: String,
    #[serde(default)]
    downloads: Vec<String>,
    #[serde(default)]
    hashes: MrpackHashes,
    #[serde(default)]
    env: Option<MrpackEnv>,
}

#[derive(Debug, Default, Deserialize)]
struct MrpackHashes {
    #[serde(default)]
    sha1: Option<String>,
    #[serde(default)]
    sha512: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MrpackEnv {
    #[serde(default)]
    client: Option<String>,
    #[serde(default)]
    server: Option<String>,
}

pub async fn install_mrpack(
    mrpack_path: &Path,
    instance_dir: &Path,
) -> Result<MrpackInstallResult, MrpackError> {
    tokio::fs::create_dir_all(instance_dir).await?;
    let mods_dir = instance_dir.join("mods");
    tokio::fs::create_dir_all(&mods_dir).await?;

    let index = read_index_from_mrpack(mrpack_path)?;
    let mut downloads = Vec::new();
    let mut mods_installed = 0usize;

    for file in &index.files {
        if file.path.starts_with("overrides/") {
            continue;
        }
        if let Some(env) = &file.env {
            if env.client.as_deref() == Some("unsupported") {
                continue;
            }
        }
        let dest = instance_dir.join(&file.path);
        if file.downloads.is_empty() {
            extract_file_from_mrpack(mrpack_path, &file.path, &dest)?;
            mods_installed += 1;
            continue;
        }
        if let Some(url) = file.downloads.first() {
            if dest.exists() {
                mods_installed += 1;
                continue;
            }
            downloads.push(DownloadItem {
                url: url.clone(),
                destination: dest,
                expected_sha1: file.hashes.sha1.clone(),
            });
            mods_installed += 1;
        }
    }

    if !downloads.is_empty() {
        ParallelDownloader::new(6).download(downloads).await?;
    }

    let overrides_copied = extract_overrides(mrpack_path, instance_dir)?;

    let loaders: Vec<String> = index
        .dependencies
        .keys()
        .filter(|k| *k != "minecraft")
        .cloned()
        .collect();

    let game_version = index
        .dependencies
        .get("minecraft")
        .cloned()
        .or(index.game)
        .unwrap_or_else(|| "unknown".into());

    Ok(MrpackInstallResult {
        mods_installed,
        overrides_copied,
        game_version,
        loaders,
    })
}

fn read_index_from_mrpack(mrpack_path: &Path) -> Result<MrpackIndex, MrpackError> {
    let file = File::open(mrpack_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut index_file = archive.by_name("modrinth.index.json")?;
    let mut buf = String::new();
    index_file.read_to_string(&mut buf)?;
    Ok(serde_json::from_str(&buf)?)
}

fn extract_file_from_mrpack(
    mrpack_path: &Path,
    name: &str,
    dest: &Path,
) -> Result<(), MrpackError> {
    let file = File::open(mrpack_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entry = archive.by_name(name)?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = File::create(dest)?;
    copy(&mut entry, &mut out)?;
    Ok(())
}

fn extract_overrides(mrpack_path: &Path, instance_dir: &Path) -> Result<usize, MrpackError> {
    let file = File::open(mrpack_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut count = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if !name.starts_with("overrides/") {
            continue;
        }
        let relative = name.strip_prefix("overrides/").unwrap_or(&name);
        if relative.is_empty() || relative.ends_with('/') {
            continue;
        }
        let dest = instance_dir.join(relative);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&dest)?;
        copy(&mut entry, &mut out)?;
        count += 1;
    }
    Ok(count)
}

pub async fn download_and_install_mrpack(
    url: &str,
    dest_mrpack: &Path,
    instance_dir: &Path,
) -> Result<MrpackInstallResult, MrpackError> {
    ParallelDownloader::new(1)
        .download(vec![DownloadItem {
            url: url.to_owned(),
            destination: dest_mrpack.to_path_buf(),
            expected_sha1: None,
        }])
        .await?;
    install_mrpack(dest_mrpack, instance_dir).await
}
