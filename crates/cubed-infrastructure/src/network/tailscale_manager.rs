use async_trait::async_trait;
use tokio::process::Command;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::{NetworkManager, TailscaleStatus};

/// Posibles ubicaciones del binario `tailscale` según plataforma.
#[cfg(target_os = "macos")]
const CANDIDATE_PATHS: &[&str] = &[
    "/usr/local/bin/tailscale",
    "/opt/homebrew/bin/tailscale",
    "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
];

#[cfg(target_os = "linux")]
const CANDIDATE_PATHS: &[&str] = &[
    "/usr/bin/tailscale",
    "/usr/local/bin/tailscale",
    "/snap/bin/tailscale",
];

#[cfg(target_os = "windows")]
const CANDIDATE_PATHS: &[&str] = &[
    r"C:\Program Files\Tailscale\tailscale.exe",
    r"C:\Program Files (x86)\Tailscale\tailscale.exe",
];

pub struct TailscaleNetworkManager;

impl TailscaleNetworkManager {
    pub fn new() -> Self { Self }

    fn find_binary() -> Option<String> {
        // 1. Try PATH first
        if which("tailscale") { return Some("tailscale".into()); }
        // 2. Try known paths
        for path in CANDIDATE_PATHS {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }
        None
    }
}

impl Default for TailscaleNetworkManager {
    fn default() -> Self { Self::new() }
}

fn which(binary: &str) -> bool {
    std::process::Command::new("which")
        .arg(binary)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[async_trait]
impl NetworkManager for TailscaleNetworkManager {
    async fn is_installed(&self) -> ApplicationResult<bool> {
        Ok(Self::find_binary().is_some())
    }

    async fn status(&self) -> ApplicationResult<TailscaleStatus> {
        let Some(bin) = Self::find_binary() else {
            return Ok(TailscaleStatus::NotInstalled);
        };

        // `tailscale status --json` gives a machine-readable output
        let out = Command::new(&bin)
            .args(["status", "--json"])
            .output()
            .await
            .map_err(|e| ApplicationError::Infrastructure(format!("tailscale status falló: {}", e)))?;

        if !out.status.success() {
            // Tailscale installed but not running / not logged in
            return Ok(TailscaleStatus::Disconnected);
        }

        let json: serde_json::Value = serde_json::from_slice(&out.stdout)
            .unwrap_or(serde_json::Value::Null);

        // BackendState: "Running" means connected
        let backend_state = json
            .get("BackendState")
            .and_then(|v| v.as_str())
            .unwrap_or("Stopped");

        if backend_state != "Running" {
            return Ok(TailscaleStatus::Disconnected);
        }

        // Self node IP and hostname
        let ip = json
            .pointer("/Self/TailscaleIPs/0")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_default();

        let hostname = json
            .pointer("/Self/HostName")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_default();

        if ip.is_empty() {
            Ok(TailscaleStatus::Disconnected)
        } else {
            Ok(TailscaleStatus::Connected { ip, hostname })
        }
    }

    async fn tailscale_ip(&self) -> ApplicationResult<Option<String>> {
        match self.status().await? {
            TailscaleStatus::Connected { ip, .. } => Ok(Some(ip)),
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn is_installed_returns_bool() {
        // Just verify it doesn't panic — result depends on the test machine
        let mgr = TailscaleNetworkManager::new();
        let result = mgr.is_installed().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn status_returns_valid_variant() {
        let mgr = TailscaleNetworkManager::new();
        let status = mgr.status().await.unwrap();
        // Must be one of the three variants — no panic, no error
        match status {
            TailscaleStatus::NotInstalled |
            TailscaleStatus::Disconnected |
            TailscaleStatus::Connected { .. } => {}
        }
    }

    #[tokio::test]
    async fn tailscale_ip_consistent_with_status() {
        let mgr = TailscaleNetworkManager::new();
        let status = mgr.status().await.unwrap();
        let ip = mgr.tailscale_ip().await.unwrap();
        match status {
            TailscaleStatus::Connected { ip: expected, .. } => {
                assert_eq!(ip, Some(expected));
            }
            _ => {
                assert!(ip.is_none());
            }
        }
    }
}
