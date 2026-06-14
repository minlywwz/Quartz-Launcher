pub mod parallel;

pub use parallel::{
    download_all, DownloadError, DownloadItem, DownloadResult, ParallelDownloader,
};
