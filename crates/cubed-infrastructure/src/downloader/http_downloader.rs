use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use cubed_domain::entities::ServerSoftware;
use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::{DownloadedJar, Downloader};

use super::url_builder;

pub struct HttpDownloader {
    client: Client,
}

impl HttpDownloader {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Cubed/0.1 (server manager)")
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    fn jar_name(software: &ServerSoftware, mc: &str) -> String {
        format!("{}-{}.jar", software.to_string().to_lowercase(), mc)
    }
}

impl Default for HttpDownloader {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Downloader for HttpDownloader {
    async fn download(
        &self,
        software: &ServerSoftware,
        minecraft_version: &str,
        dest_dir: &str,
    ) -> ApplicationResult<DownloadedJar> {
        fs::create_dir_all(dest_dir)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        let url = url_builder::resolve_url(&self.client, software, minecraft_version).await?;
        let dest_path = Path::new(dest_dir)
            .join(Self::jar_name(software, minecraft_version));

        let resp = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| ApplicationError::Infrastructure(
                format!("Error de red al descargar {}: {}", url, e),
            ))?;

        if !resp.status().is_success() {
            return Err(ApplicationError::Infrastructure(format!(
                "HTTP {} al descargar {}",
                resp.status(),
                url
            )));
        }

        let mut file = fs::File::create(&dest_path)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        let mut stream = resp.bytes_stream();
        let mut size_bytes: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
            size_bytes += chunk.len() as u64;
            file.write_all(&chunk)
                .await
                .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        }

        Ok(DownloadedJar {
            path: dest_path.to_string_lossy().to_string(),
            size_bytes,
        })
    }

    fn build_url(&self, software: &ServerSoftware, minecraft_version: &str) -> ApplicationResult<String> {
        url_builder::static_url(software, minecraft_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jar_name_format() {
        assert_eq!(HttpDownloader::jar_name(&ServerSoftware::Paper, "1.21.4"), "paper-1.21.4.jar");
        assert_eq!(HttpDownloader::jar_name(&ServerSoftware::Purpur, "1.21.4"), "purpur-1.21.4.jar");
        assert_eq!(HttpDownloader::jar_name(&ServerSoftware::Fabric, "1.21.4"), "fabric-1.21.4.jar");
    }

    #[test]
    fn build_url_purpur_is_static() {
        let dl = HttpDownloader::new();
        let url = dl.build_url(&ServerSoftware::Purpur, "1.21.4").unwrap();
        assert!(url.contains("purpur"));
        assert!(url.contains("1.21.4"));
    }

    #[test]
    fn build_url_paper_requires_network() {
        let dl = HttpDownloader::new();
        assert!(dl.build_url(&ServerSoftware::Paper, "1.21.4").is_err());
    }
}
