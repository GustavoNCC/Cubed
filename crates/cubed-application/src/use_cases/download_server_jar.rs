use std::sync::Arc;
use cubed_domain::entities::ServerSoftware;
use crate::error::ApplicationResult;
use crate::ports::{DownloadedJar, Downloader};

pub struct DownloadServerJar {
    downloader: Arc<dyn Downloader>,
}

impl DownloadServerJar {
    pub fn new(downloader: Arc<dyn Downloader>) -> Self {
        Self { downloader }
    }

    pub async fn execute(
        &self,
        software: &ServerSoftware,
        minecraft_version: &str,
        dest_dir: &str,
    ) -> ApplicationResult<DownloadedJar> {
        self.downloader.download(software, minecraft_version, dest_dir).await
    }

    pub fn preview_url(
        &self,
        software: &ServerSoftware,
        minecraft_version: &str,
    ) -> ApplicationResult<String> {
        self.downloader.build_url(software, minecraft_version)
    }
}
