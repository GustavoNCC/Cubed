import { invoke } from "@tauri-apps/api/core";
import type { BackupDto, Server, CreateServerForm, SystemStats, ServerStats, TailscaleStatusDto } from "./types";

export const api = {
  listServers: () => invoke<Server[]>("list_servers"),
  createServer: (form: CreateServerForm) => invoke<Server>("create_server", { cmd: form }),
  startServer: (id: string) => invoke<Server>("start_server", { id }),
  stopServer: (id: string) => invoke<Server>("stop_server", { id }),
  deleteServer: (id: string) => invoke<void>("delete_server", { id }),
  getSystemStats: () => invoke<SystemStats>("get_system_stats"),
  getServerStats: (id: string, pid: number) => invoke<ServerStats | null>("get_server_stats", { id, pid }),
  listBackups: (serverId: string) => invoke<BackupDto[]>("list_backups", { serverId }),
  createBackup: (serverId: string, serverName: string, serverDir: string) =>
    invoke<BackupDto>("create_backup", { serverId, serverName, serverDir }),
  restoreBackup: (backupId: string, restoreDir: string) =>
    invoke<void>("restore_backup", { backupId, restoreDir }),
  deleteBackup: (backupId: string, deleteFile = true) =>
    invoke<void>("delete_backup", { backupId, deleteFile }),
  // Network / Tailscale
  tailscaleIsInstalled: () => invoke<boolean>("tailscale_is_installed"),
  tailscaleStatus: () => invoke<TailscaleStatusDto>("tailscale_status"),
  tailscaleIp: () => invoke<string | null>("tailscale_ip"),
  serverConnectAddress: (serverId: string) => invoke<string | null>("server_connect_address", { serverId }),
};
