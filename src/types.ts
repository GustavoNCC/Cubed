export type ServerStatus = "offline" | "starting" | "running" | "stopping" | "crashed";

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
