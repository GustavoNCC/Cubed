use reqwest::Client;
use serde::Deserialize;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_domain::entities::ServerSoftware;

// ── Paper ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PaperBuilds {
    builds: Vec<PaperBuild>,
}

#[derive(Deserialize)]
struct PaperBuild {
    build: u32,
    downloads: PaperDownloads,
}

#[derive(Deserialize)]
struct PaperDownloads {
    application: PaperApplication,
}

#[derive(Deserialize)]
struct PaperApplication {
    name: String,
}

async fn paper_url(client: &Client, mc: &str) -> ApplicationResult<String> {
    let builds_url = format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}/builds",
        mc
    );
    let resp: PaperBuilds = client
        .get(&builds_url)
        .send()
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?
        .json()
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

    let latest = resp
        .builds
        .into_iter()
        .max_by_key(|b| b.build)
        .ok_or_else(|| {
            ApplicationError::Infrastructure(format!(
                "No hay builds de Paper para Minecraft {}",
                mc
            ))
        })?;

    Ok(format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/{}",
        mc, latest.build, latest.downloads.application.name
    ))
}

// ── Purpur ────────────────────────────────────────────────────────────────────

fn purpur_url(mc: &str) -> String {
    format!("https://api.purpurmc.org/v2/purpur/{}/latest/download", mc)
}

// ── Fabric ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct FabricLoader {
    loader: FabricLoaderVersion,
}

#[derive(Deserialize)]
struct FabricLoaderVersion {
    version: String,
}

#[derive(Deserialize)]
struct FabricInstaller {
    version: String,
}

async fn fabric_url(client: &Client, mc: &str) -> ApplicationResult<String> {
    // Último loader
    let loaders: Vec<FabricLoader> = client
        .get(format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}",
            mc
        ))
        .send()
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?
        .json()
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

    let loader_version = loaders
        .into_iter()
        .next()
        .ok_or_else(|| {
            ApplicationError::Infrastructure(format!(
                "No hay versiones de Fabric loader para Minecraft {}",
                mc
            ))
        })?
        .loader
        .version;

    // Último installer
    let installers: Vec<FabricInstaller> = client
        .get("https://meta.fabricmc.net/v2/versions/installer")
        .send()
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?
        .json()
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

    let installer_version = installers
        .into_iter()
        .next()
        .ok_or_else(|| {
            ApplicationError::Infrastructure("No hay versiones de Fabric installer".to_string())
        })?
        .version;

    Ok(format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/{}/server/jar",
        mc, loader_version, installer_version
    ))
}

// ── Forge ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ForgePromos {
    promos: std::collections::HashMap<String, String>,
}

async fn forge_url(client: &Client, mc: &str) -> ApplicationResult<String> {
    let promos: ForgePromos = client
        .get("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json")
        .send()
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?
        .json()
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

    // Preferir -recommended, caer en -latest
    let forge_ver = promos
        .promos
        .get(&format!("{}-recommended", mc))
        .or_else(|| promos.promos.get(&format!("{}-latest", mc)))
        .ok_or_else(|| {
            ApplicationError::Infrastructure(format!(
                "No hay versión de Forge para Minecraft {}",
                mc
            ))
        })?
        .clone();

    let full = format!("{}-{}", mc, forge_ver);
    Ok(format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{}/forge-{}-installer.jar",
        full, full
    ))
}

// ── NeoForge ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct NeoForgeVersions {
    versions: Vec<String>,
}

async fn neoforge_url(client: &Client, mc: &str) -> ApplicationResult<String> {
    // NeoForge usa el esquema mc-menor.mc-parche.neoforge-parche, p. ej. 21.1.1
    // La API de Maven lista las versiones disponibles
    let mc_minor = mc.split('.').nth(1).unwrap_or("0");
    let mc_patch = mc.split('.').nth(2).unwrap_or("0");
    let neo_prefix = format!("{}.{}.", mc_minor, mc_patch);

    let meta_url = format!(
        "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge?filter={}",
        neo_prefix
    );

    let resp: NeoForgeVersions = client
        .get(&meta_url)
        .send()
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?
        .json()
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

    let latest = resp.versions.into_iter().last().ok_or_else(|| {
        ApplicationError::Infrastructure(format!(
            "No hay versiones de NeoForge para Minecraft {}",
            mc
        ))
    })?;

    Ok(format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
        latest, latest
    ))
}

// ── Punto de entrada ──────────────────────────────────────────────────────────

/// Construye la URL de descarga consultando la API correspondiente.
pub async fn resolve_url(
    client: &Client,
    software: &ServerSoftware,
    mc: &str,
) -> ApplicationResult<String> {
    match software {
        ServerSoftware::Paper => paper_url(client, mc).await,
        ServerSoftware::Purpur => Ok(purpur_url(mc)),
        ServerSoftware::Fabric => fabric_url(client, mc).await,
        ServerSoftware::Forge => forge_url(client, mc).await,
        ServerSoftware::NeoForge => neoforge_url(client, mc).await,
    }
}

/// Construye una URL sin hacer peticiones de red (solo para software con URLs estáticas).
/// Para Paper/Fabric/Forge/NeoForge retorna un error indicando que se necesita red.
pub fn static_url(software: &ServerSoftware, mc: &str) -> ApplicationResult<String> {
    match software {
        ServerSoftware::Purpur => Ok(purpur_url(mc)),
        other => Err(ApplicationError::Infrastructure(format!(
            "{} requiere resolución de URL en línea",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purpur_url_format() {
        let url = purpur_url("1.21.4");
        assert!(url.contains("1.21.4"));
        assert!(url.starts_with("https://"));
    }

    #[test]
    fn static_url_purpur_ok() {
        assert!(static_url(&ServerSoftware::Purpur, "1.21.4").is_ok());
    }

    #[test]
    fn static_url_paper_needs_network() {
        assert!(static_url(&ServerSoftware::Paper, "1.21.4").is_err());
    }
}
