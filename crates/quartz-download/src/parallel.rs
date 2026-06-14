use std::path::PathBuf;

use reqwest::Client;
use sha1::{Digest, Sha1};
use thiserror::Error;
use tokio::task::JoinSet;

#[derive(Debug, Clone)]
pub struct DownloadItem {
    pub url: String,
    pub destination: PathBuf,

    pub expected_sha1: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub url: String,
    pub destination: PathBuf,
    pub bytes: usize,
    pub sha1: String,
}

#[derive(Debug, Default)]
pub struct ParallelDownloader {
    client: Client,
    concurrency: usize,
}

impl ParallelDownloader {
    pub fn new(concurrency: usize) -> Self {
        Self {
            client: Client::new(),
            concurrency: concurrency.max(1),
        }
    }

    pub async fn download(&self, items: Vec<DownloadItem>) -> Result<Vec<DownloadResult>, DownloadError> {
        download_all(&self.client, items, self.concurrency).await
    }
}

pub async fn download_all(
    client: &Client,
    items: Vec<DownloadItem>,
    concurrency: usize,
) -> Result<Vec<DownloadResult>, DownloadError> {
    let mut results = Vec::with_capacity(items.len());
    let mut join_set = JoinSet::new();
    let mut pending = items.into_iter();

    for _ in 0..concurrency.max(1) {
        if let Some(item) = pending.next() {
            let client = client.clone();
            join_set.spawn(async move { fetch_one(&client, item).await });
        }
    }

    while let Some(joined) = join_set.join_next().await {
        results.push(joined??);
        if let Some(item) = pending.next() {
            let client = client.clone();
            join_set.spawn(async move { fetch_one(&client, item).await });
        }
    }

    Ok(results)
}

async fn fetch_one(client: &Client, item: DownloadItem) -> Result<DownloadResult, DownloadError> {
    let bytes = client.get(&item.url).send().await?.bytes().await?;
    let sha1_hex = hex_sha1(&bytes);

    if let Some(expected) = &item.expected_sha1 {
        if !expected.eq_ignore_ascii_case(&sha1_hex) {
            return Err(DownloadError::ChecksumMismatch {
                url: item.url.clone(),
                expected: expected.clone(),
                actual: sha1_hex,
            });
        }
    }

    if let Some(parent) = item.destination.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&item.destination, &bytes).await?;

    Ok(DownloadResult {
        url: item.url,
        destination: item.destination,
        bytes: bytes.len(),
        sha1: sha1_hex,
    })
}

fn hex_sha1(bytes: &[u8]) -> String {
    let digest = Sha1::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("checksum mismatch for {url}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        url: String,
        expected: String,
        actual: String,
    },
}
