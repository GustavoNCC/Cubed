import { invoke } from "@tauri-apps/api/core";
import type { Server, CreateServerForm, SystemStats, ServerStats } from "./types";

export const api = {
  listServers: () => invoke<Server[]>("list_servers"),
  createServer: (form: CreateServerForm) => invoke<Server>("create_server", { cmd: form }),
  startServer: (id: string) => invoke<Server>("start_server", { id }),
  stopServer: (id: string) => invoke<Server>("stop_server", { id }),
  deleteServer: (id: string) => invoke<void>("delete_server", { id }),
  getSystemStats: () => invoke<SystemStats>("get_system_stats"),
  getServerStats: (id: string, pid: number) => invoke<ServerStats | null>("get_server_stats", { id, pid }),
};
