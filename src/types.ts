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
