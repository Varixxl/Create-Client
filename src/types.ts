export interface AppConfig {
  ram_gb: number;
  jvm_flags: string;
  game_dir: string;
  azure_client_id: string;
  auto_connect: boolean;
  release_manifest_url: string;
}

export interface MinecraftProfile {
  uuid: string;
  username: string;
  skin_url?: string;
  access_token: string;
  refresh_token: string;
  expires_at: number;
}

export interface AuthStatus {
  is_authenticated: boolean;
  profile: MinecraftProfile | null;
}

export interface ServerStatus {
  server_name: string;
  online: boolean;
  ping_ms: number;
  online_players: number;
  max_players: number;
  motd: string;
  version: string;
}

export interface ProgressPayload {
  task: string;
  current_bytes: number;
  total_bytes: number;
  percentage: number;
  speed_bytes_sec: number;
}

export interface DeviceCodeInfo {
  user_code: string;
  device_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export interface ClientUpdateInfo {
  has_update: boolean;
  current_version: string;
  latest_version: string;
  download_url: string | null;
  asset_name: string | null;
  release_url: string;
  changelog: string | null;
}

export interface ModpackUpdateInfo {
  has_update: boolean;
  current_version: string;
  latest_version: string;
  changelog: string | null;
}
