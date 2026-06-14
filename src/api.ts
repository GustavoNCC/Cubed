import { invoke } from "@tauri-apps/api/core";
import type {
  BackupDto,
  ConsoleLine,
  InstallSummaryDto,
  JavaInstallationDto,
  ModDto,
  ModpackDto,
  Server,
  CreateServerForm,
  SystemStats,
  ServerStats,
  TailscaleStatusDto,
  SettingsDto,
} from "./types";

export const api = {
  listServers: () => invoke<Server[]>("list_servers"),
  createServer: (form: CreateServerForm) =>
    invoke<Server>("create_server", { cmd: form }),
  startServer: (id: string) => invoke<Server>("start_server", { id }),
  stopServer: (id: string) => invoke<Server>("stop_server", { id }),
  restartServer: (id: string) => invoke<Server>("restart_server", { id }),
  deleteServer: (id: string) => invoke<void>("delete_server", { id }),
  subscribeConsole: (id: string) =>
    invoke<ConsoleLine[]>("subscribe_console", { id }),
  sendConsoleCommand: (id: string, command: string) =>
    invoke<void>("send_console_command", { id, command }),
  getSystemStats: () => invoke<SystemStats>("get_system_stats"),
  getServerStats: (id: string, pid: number) =>
    invoke<ServerStats | null>("get_server_stats", { id, pid }),
  listBackups: (serverId: string) =>
    invoke<BackupDto[]>("list_backups", { serverId }),
  createBackup: (serverId: string, serverName: string, serverDir: string) =>
    invoke<BackupDto>("create_backup", { serverId, serverName, serverDir }),
  restoreBackup: (backupId: string, restoreDir: string) =>
    invoke<void>("restore_backup", { backupId, restoreDir }),
  deleteBackup: (backupId: string, deleteFile = true) =>
    invoke<void>("delete_backup", { backupId, deleteFile }),
  listMods: (serverId: string) => invoke<ModDto[]>("list_mods", { serverId }),
  validateJar: (path: string) => invoke<boolean>("validate_jar", { path }),
  installMod: (serverId: string, sourcePath: string, modsDir: string) =>
    invoke<ModDto>("install_mod", { serverId, sourcePath, modsDir }),
  removeMod: (modId: string) => invoke<void>("remove_mod", { modId }),
  listModpacks: (serverId: string) =>
    invoke<ModpackDto[]>("list_modpacks", { serverId }),
  installModpack: (serverId: string, sourcePath: string, installDir: string) =>
    invoke<InstallSummaryDto>("install_modpack", {
      serverId,
      sourcePath,
      installDir,
    }),
  deleteModpack: (modpackId: string) =>
    invoke<void>("delete_modpack", { modpackId }),
  suggestFreePort: () => invoke<number>("suggest_free_port"),
  // Network / Tailscale
  tailscaleIsInstalled: () => invoke<boolean>("tailscale_is_installed"),
  tailscaleStatus: () => invoke<TailscaleStatusDto>("tailscale_status"),
  tailscaleIp: () => invoke<string | null>("tailscale_ip"),
  serverConnectAddress: (serverId: string) =>
    invoke<string | null>("server_connect_address", { serverId }),
  // Java detection
  detectJava: () => invoke<JavaInstallationDto[]>("detect_java"),
  selectJavaForVersion: (mcVersion: string) =>
    invoke<JavaInstallationDto>("select_java_for_version", { mcVersion }),
  // Settings
  getSettings: () => invoke<SettingsDto>("get_settings"),
  saveSettings: (cmd: Omit<SettingsDto, never>) =>
    invoke<SettingsDto>("save_settings", { cmd }),
};
