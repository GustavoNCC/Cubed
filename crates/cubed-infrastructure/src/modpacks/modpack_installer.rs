use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;
use tokio::fs;
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::{ModpackRepository, ServerRepository};
use cubed_application::use_cases::{ImportModpack, ImportModpackInput};
use cubed_domain::entities::{Modpack, ModpackFormat};

// ── Modrinth .mrpack manifest ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct MrpackIndex {
    name: Option<String>,
    files: Vec<MrpackFile>,
    dependencies: Option<std::collections::HashMap<String, String>>,
}

#[derive(Deserialize)]
struct MrpackFile {
    path: String,
    downloads: Vec<String>,
    #[serde(default)]
    env: Option<MrpackEnv>,
}

#[derive(Deserialize)]
struct MrpackEnv {
    server: Option<String>,
}

// ── CurseForge-style .zip manifest ───────────────────────────────────────────

#[derive(Deserialize)]
struct CfManifest {
    name: Option<String>,
    files: Vec<CfFile>,
}

#[derive(Deserialize)]
struct CfFile {
    #[serde(rename = "projectID")]
    project_id: Option<u64>,
    #[serde(rename = "fileID")]
    file_id: Option<u64>,
    #[serde(default)]
    url: Option<String>,
}

// ── Progress report ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct InstallProgress {
    pub total: usize,
    pub done: usize,
    pub current_file: String,
}

// ── Installer ─────────────────────────────────────────────────────────────────

pub struct ModpackInstaller {
    servers:  Arc<dyn ServerRepository>,
    modpacks: Arc<dyn ModpackRepository>,
}

impl ModpackInstaller {
    pub fn new(servers: Arc<dyn ServerRepository>, modpacks: Arc<dyn ModpackRepository>) -> Arc<Self> {
        Arc::new(Self { servers, modpacks })
    }

    pub async fn install(
        &self,
        server_id: Uuid,
        source_path: &str,
        install_dir: &str,
        progress_cb: impl Fn(InstallProgress) + Send + Sync + 'static,
    ) -> ApplicationResult<(Modpack, InstallSummary)> {
        let uc = ImportModpack::new(self.servers.clone(), self.modpacks.clone());
        let modpack = uc.execute(ImportModpackInput {
            server_id,
            source_path: source_path.to_string(),
        }).await?;

        // Wrap in Arc so it can be cloned into spawn_blocking closures
        let cb: Arc<dyn Fn(InstallProgress) + Send + Sync + 'static> = Arc::new(progress_cb);

        let summary = match modpack.format() {
            ModpackFormat::Mrpack => self.install_mrpack(source_path, install_dir, cb).await?,
            ModpackFormat::Zip    => self.install_zip(source_path, install_dir, cb).await?,
        };

        Ok((modpack, summary))
    }

    async fn install_mrpack(
        &self,
        source_path: &str,
        install_dir: &str,
        progress_cb: Arc<dyn Fn(InstallProgress) + Send + Sync + 'static>,
    ) -> ApplicationResult<InstallSummary> {
        // Parse manifest on blocking thread pool — zip crate is synchronous
        let sp = source_path.to_string();
        let index = tokio::task::spawn_blocking(move || read_mrpack_index(&sp))
            .await
            .map_err(|e| ApplicationError::Infrastructure(format!("spawn_blocking: {}", e)))??;

        let files: Vec<MrpackFile> = index.files.into_iter()
            .filter(|f| {
                f.env.as_ref()
                    .and_then(|e| e.server.as_deref())
                    .map(|s| s != "unsupported")
                    .unwrap_or(true)
            })
            .collect();

        let total = files.len();
        let mut downloaded = 0usize;
        let mut skipped = 0usize;
        let mods_dir = format!("{}/mods", install_dir);
        fs::create_dir_all(&mods_dir).await.map_err(|e| {
            ApplicationError::Infrastructure(format!("No se pudo crear mods/: {}", e))
        })?;

        let client = reqwest::Client::new();
        for file in &files {
            let file_name = Path::new(&file.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file.path);

            progress_cb(InstallProgress {
                total,
                done: downloaded,
                current_file: file_name.to_string(),
            });

            let mut success = false;
            for url in &file.downloads {
                match download_file(&client, url, &mods_dir, file_name).await {
                    Ok(()) => { success = true; break; }
                    Err(_) => continue,
                }
            }
            if success { downloaded += 1; } else { skipped += 1; }
        }

        let loader_info = index.dependencies.map(|d| {
            d.iter()
                .filter(|(k, _)| k.as_str() != "minecraft")
                .map(|(k, v)| format!("{k} {v}"))
                .collect::<Vec<_>>()
                .join(", ")
        });

        Ok(InstallSummary {
            name: index.name.unwrap_or_else(|| "Modpack".into()),
            total_files: total,
            downloaded,
            skipped,
            loader_info,
        })
    }

    async fn install_zip(
        &self,
        source_path: &str,
        install_dir: &str,
        progress_cb: Arc<dyn Fn(InstallProgress) + Send + Sync + 'static>,
    ) -> ApplicationResult<InstallSummary> {
        let mods_dir = format!("{}/mods", install_dir);
        fs::create_dir_all(&mods_dir).await.map_err(|e| {
            ApplicationError::Infrastructure(format!("No se pudo crear mods/: {}", e))
        })?;

        // Parse CF manifest on blocking thread
        let sp = source_path.to_string();
        let cf_result = tokio::task::spawn_blocking(move || read_cf_manifest(&sp))
            .await
            .map_err(|e| ApplicationError::Infrastructure(format!("spawn_blocking: {}", e)))?;

        match cf_result {
            Ok(manifest) => {
                let total = manifest.files.len();
                let mut downloaded = 0usize;
                let mut skipped = 0usize;
                let client = reqwest::Client::new();

                for file in &manifest.files {
                    let name = format!("{}-{}.jar",
                        file.project_id.unwrap_or(0),
                        file.file_id.unwrap_or(0),
                    );
                    progress_cb(InstallProgress {
                        total, done: downloaded, current_file: name.clone(),
                    });

                    if let Some(url) = &file.url {
                        match download_file(&client, url, &mods_dir, &name).await {
                            Ok(()) => downloaded += 1,
                            Err(_) => skipped += 1,
                        }
                    } else {
                        skipped += 1;
                    }
                }

                Ok(InstallSummary {
                    name: manifest.name.unwrap_or_else(|| "Modpack".into()),
                    total_files: total,
                    downloaded,
                    skipped,
                    loader_info: None,
                })
            }
            Err(_) => {
                // Fallback: extract all .jar files from the zip into mods/
                let sp2 = source_path.to_string();
                let md = mods_dir.clone();
                let cb2 = progress_cb.clone();
                tokio::task::spawn_blocking(move || extract_jars_from_zip(&sp2, &md, &*cb2))
                    .await
                    .map_err(|e| ApplicationError::Infrastructure(format!("spawn_blocking: {}", e)))??;

                let count = count_jars(&mods_dir).await;
                Ok(InstallSummary {
                    name: Path::new(source_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Modpack")
                        .to_string(),
                    total_files: count,
                    downloaded: count,
                    skipped: 0,
                    loader_info: None,
                })
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallSummary {
    pub name: String,
    pub total_files: usize,
    pub downloaded: usize,
    pub skipped: usize,
    pub loader_info: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_mrpack_index(path: &str) -> ApplicationResult<MrpackIndex> {
    let file = std::fs::File::open(path).map_err(|e| {
        ApplicationError::Infrastructure(format!("No se pudo abrir '{}': {}", path, e))
    })?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| {
        ApplicationError::Infrastructure(format!("No es un ZIP válido: {}", e))
    })?;
    let mut entry = zip.by_name("modrinth.index.json").map_err(|_| {
        ApplicationError::Infrastructure("modrinth.index.json no encontrado en el .mrpack".into())
    })?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf).map_err(|e| {
        ApplicationError::Infrastructure(format!("Error leyendo manifest: {}", e))
    })?;
    serde_json::from_str(&buf).map_err(|e| {
        ApplicationError::Infrastructure(format!("Manifest JSON inválido: {}", e))
    })
}

fn read_cf_manifest(path: &str) -> ApplicationResult<CfManifest> {
    let file = std::fs::File::open(path).map_err(|e| {
        ApplicationError::Infrastructure(format!("No se pudo abrir '{}': {}", path, e))
    })?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| {
        ApplicationError::Infrastructure(format!("No es un ZIP válido: {}", e))
    })?;
    let mut entry = zip.by_name("manifest.json").map_err(|_| {
        ApplicationError::Infrastructure("manifest.json no encontrado".into())
    })?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf).map_err(|e| {
        ApplicationError::Infrastructure(format!("Error leyendo manifest: {}", e))
    })?;
    serde_json::from_str(&buf).map_err(|e| {
        ApplicationError::Infrastructure(format!("Manifest JSON inválido: {}", e))
    })
}

fn extract_jars_from_zip(
    zip_path: &str,
    dest_dir: &str,
    progress_cb: &(dyn Fn(InstallProgress) + Send + Sync),
) -> ApplicationResult<()> {
    let file = std::fs::File::open(zip_path).map_err(|e| {
        ApplicationError::Infrastructure(format!("No se pudo abrir ZIP: {}", e))
    })?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| {
        ApplicationError::Infrastructure(format!("ZIP inválido: {}", e))
    })?;

    let total = zip.len();
    let mut done = 0;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| {
            ApplicationError::Infrastructure(format!("Error leyendo entrada ZIP: {}", e))
        })?;
        let name = entry.name().to_string();
        if !name.ends_with(".jar") { continue; }

        let file_name = Path::new(&name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&name)
            .to_string();

        progress_cb(InstallProgress { total, done, current_file: file_name.clone() });

        let dest = format!("{}/{}", dest_dir, file_name);
        let mut out = std::fs::File::create(&dest).map_err(|e| {
            ApplicationError::Infrastructure(format!("Error creando '{}': {}", dest, e))
        })?;
        std::io::copy(&mut entry, &mut out).map_err(|e| {
            ApplicationError::Infrastructure(format!("Error extrayendo '{}': {}", file_name, e))
        })?;
        done += 1;
    }
    Ok(())
}

async fn download_file(client: &reqwest::Client, url: &str, dir: &str, file_name: &str) -> ApplicationResult<()> {
    let resp = client.get(url).send().await.map_err(|e| {
        ApplicationError::Infrastructure(format!("Error descargando '{}': {}", url, e))
    })?;
    if !resp.status().is_success() {
        return Err(ApplicationError::Infrastructure(
            format!("HTTP {} al descargar '{}'", resp.status(), url),
        ));
    }
    let bytes = resp.bytes().await.map_err(|e| {
        ApplicationError::Infrastructure(format!("Error leyendo respuesta: {}", e))
    })?;
    let dest = format!("{}/{}", dir, file_name);
    fs::write(&dest, &bytes).await.map_err(|e| {
        ApplicationError::Infrastructure(format!("Error guardando '{}': {}", dest, e))
    })
}

async fn count_jars(dir: &str) -> usize {
    let mut count = 0usize;
    if let Ok(mut rd) = fs::read_dir(dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            if entry.file_name().to_string_lossy().ends_with(".jar") {
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpacks::InMemoryModpackRepo;
    use crate::persistence::InMemoryServerRepo;
    use cubed_domain::entities::ServerSoftware;
    use cubed_domain::value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};

    fn make_server() -> cubed_domain::entities::Server {
        cubed_domain::entities::Server::new(
            ServerName::new("srv").unwrap(),
            ServerVersion::new("1.21").unwrap(),
            ServerSoftware::Paper,
            ServerPort::new(25565).unwrap(),
            JavaPath::new("/usr/bin/java").unwrap(),
        )
    }

    #[tokio::test]
    async fn install_invalid_path_returns_error() {
        let srv = make_server();
        let sid = srv.id();
        let servers = InMemoryServerRepo::new();
        servers.save(&srv).await.unwrap();
        let repo = InMemoryModpackRepo::new();
        let installer = ModpackInstaller::new(servers, repo);
        let result = installer.install(sid, "/no/such.mrpack", "/tmp", |_| {}).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn install_unsupported_format_fails() {
        let srv = make_server();
        let sid = srv.id();
        let servers = InMemoryServerRepo::new();
        servers.save(&srv).await.unwrap();
        let repo = InMemoryModpackRepo::new();
        let installer = ModpackInstaller::new(servers, repo);
        let result = installer.install(sid, "/pack.tar.gz", "/tmp", |_| {}).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn install_real_mrpack() {
        use tempfile::tempdir;
        use std::io::Write;

        let dir = tempdir().unwrap();
        let pack_path = dir.path().join("test.mrpack");
        let manifest = r#"{
            "name": "TestPack",
            "files": [],
            "dependencies": {"minecraft": "1.21", "fabric-loader": "0.15.0"}
        }"#;
        {
            let f = std::fs::File::create(&pack_path).unwrap();
            let mut zip = zip::ZipWriter::new(f);
            zip.start_file("modrinth.index.json", zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(manifest.as_bytes()).unwrap();
            zip.finish().unwrap();
        }

        let srv = make_server();
        let sid = srv.id();
        let servers = InMemoryServerRepo::new();
        servers.save(&srv).await.unwrap();
        let repo = InMemoryModpackRepo::new();
        let install_dir = tempdir().unwrap();
        let installer = ModpackInstaller::new(servers, repo.clone());

        let (mp, summary) = installer.install(
            sid,
            pack_path.to_str().unwrap(),
            install_dir.path().to_str().unwrap(),
            |_| {},
        ).await.unwrap();

        assert_eq!(mp.format(), &ModpackFormat::Mrpack);
        assert_eq!(summary.name, "TestPack");
        assert_eq!(summary.total_files, 0);
        assert!(summary.loader_info.as_deref().unwrap_or("").contains("fabric-loader"));
        assert!(repo.find_by_id(mp.id()).await.unwrap().is_some());
    }
}
