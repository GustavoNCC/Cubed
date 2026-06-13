export type ServerStatus =
  | "offline"
  | "starting"
  | "running"
  | "stopping"
  | "crashed";

export interface Server {
  id: string;
  name: string;
  version: string;
  software: string;
  port: number;
  status: ServerStatus;
}

export interface CreateServerForm {
  name: string;
  version: string;
  software: string;
  port: number;
  java_path: string;
  servers_dir: string;
}

export interface SystemStats {
  cpu_percent: number;
  ram_used_bytes: number;
  ram_total_bytes: number;
  disk_used_bytes: number;
  disk_total_bytes: number;
  net_rx_bytes: number;
  net_tx_bytes: number;
}

export interface ServerStats {
  server_id: string;
  cpu_percent: number;
  ram_bytes: number;
  uptime_secs: number;
}

export interface TailscaleStatusDto {
  state: "not_installed" | "disconnected" | "connected";
  ip: string | null;
  hostname: string | null;
}

export interface JavaInstallationDto {
  path: string;
  major_version: number;
  version_string: string;
}

export interface SettingsDto {
  servers_dir: string;
  backups_dir: string;
  downloads_dir: string;
  default_java_path: string;
  /** Intervalo de backup automático en segundos. 0 = desactivado. */
  backup_interval_secs: number;
}

export interface BackupDto {
  id: string;
  server_id: string;
  path: string;
  size_bytes: number;
  created_at: string;
}
