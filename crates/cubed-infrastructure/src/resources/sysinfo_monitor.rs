use async_trait::async_trait;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Disks, Networks, System};
use tokio::sync::Mutex;
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::{ResourceMonitor, ServerStats, SystemStats};

pub struct SysInfoResourceMonitor {
    sys: Mutex<System>,
}

impl SysInfoResourceMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self { sys: Mutex::new(sys) }
    }
}

impl Default for SysInfoResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ResourceMonitor for SysInfoResourceMonitor {
    async fn system_stats(&self) -> ApplicationResult<SystemStats> {
        let mut sys = self.sys.lock().await;
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let cpu_percent = sys.global_cpu_usage();
        let ram_used_bytes  = sys.used_memory();
        let ram_total_bytes = sys.total_memory();

        let disks = Disks::new_with_refreshed_list();
        let (disk_total_bytes, disk_used_bytes) = disks
            .list()
            .iter()
            .fold((0u64, 0u64), |(total, used), d| {
                (total + d.total_space(), used + (d.total_space() - d.available_space()))
            });

        let networks = Networks::new_with_refreshed_list();
        let (net_rx_bytes, net_tx_bytes) = networks
            .iter()
            .fold((0u64, 0u64), |(rx, tx), (_, data)| {
                (rx + data.total_received(), tx + data.total_transmitted())
            });

        Ok(SystemStats {
            cpu_percent,
            ram_used_bytes,
            ram_total_bytes,
            disk_used_bytes,
            disk_total_bytes,
            net_rx_bytes,
            net_tx_bytes,
        })
    }

    async fn server_stats(&self, server_id: Uuid, pid: u32) -> ApplicationResult<Option<ServerStats>> {
        let mut sys = self.sys.lock().await;
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let sysinfo_pid = sysinfo::Pid::from_u32(pid);
        let Some(proc) = sys.process(sysinfo_pid) else {
            return Ok(None);
        };

        let start_unix = proc.start_time();
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?
            .as_secs();
        let uptime_secs = now_unix.saturating_sub(start_unix);

        Ok(Some(ServerStats {
            server_id,
            cpu_percent: proc.cpu_usage(),
            ram_bytes: proc.memory(),
            uptime_secs,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn system_stats_returns_nonzero_total_ram() {
        let monitor = SysInfoResourceMonitor::new();
        let stats = monitor.system_stats().await.unwrap();
        assert!(stats.ram_total_bytes > 0, "total RAM must be > 0");
        assert!(stats.cpu_percent >= 0.0 && stats.cpu_percent <= 100.0 * 256.0);
    }

    #[tokio::test]
    async fn server_stats_nonexistent_pid_returns_none() {
        let monitor = SysInfoResourceMonitor::new();
        // PID 0 nunca existe como proceso de usuario
        let result = monitor.server_stats(Uuid::new_v4(), 0).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn system_stats_disk_populated() {
        let monitor = SysInfoResourceMonitor::new();
        let stats = monitor.system_stats().await.unwrap();
        assert!(stats.disk_total_bytes > 0, "disk total must be > 0");
    }
}
