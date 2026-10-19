export interface AppConfig {
  ram_gb: number;
  jvm_flags: string;
  game_dir: string;
  azure_client_id: string;
  auto_connect: boolean;
  release_manifest_url: string;
  graphics_preset: string;
  curseforge_api_key: string;
}

export interface GraphicsInfo {
  detected_gpu: string;
  detected_tier: string;
  detected_refresh_rate: number;
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

export interface ModpackProfile {
  id: string;
  name: string;
  description: string;
  loader: string;
  mc_version: string;
  resolved_mc_version?: string | null;
  loader_version?: string | null;
  is_builtin: boolean;
  instance_dir: string;
  modrinth_slugs: string[];
  ram_gb?: number | null;
  jvm_flags?: string | null;
  java_version?: number | null;
}

export interface ProfilesState {
  selected_profile_id: string;
  profiles: ModpackProfile[];
}

export interface InstalledModInfo {
  file_name: string;
  title: string;
  slug?: string | null;
  version_number: string;
  description: string;
  icon_url?: string | null;
  enabled: boolean;
  size_bytes: number;
}

export type ContentType = "mod" | "resourcepack" | "shader";
export type ContentSource = "modrinth" | "curseforge";

export interface ContentSearchResult {
  project_id: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  downloads: number;
  icon_url?: string | null;
  installed: boolean;
  content_type: ContentType;
  source: ContentSource;
}

export interface ModrinthSearchResult {
  project_id: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  downloads: number;
  icon_url?: string | null;
  installed: boolean;
}

export interface ModloaderVersionItem {
  version: string;
  stable: boolean;
  recommended: boolean;
}

export interface ServerInstance {
  id: string;
  profile_id: string;
  name: string;
  server_dir: string;
  mc_version: string;
  loader: string;
  loader_version: string;
  host: string;
  created_at: string;
  installed_mods: string[];
}

export interface ServerDivergenceInfo {
  server_id: string;
  server_name: string;
  profile_mods_count: number;
  server_mods_count: number;
  added_to_profile: string[];
  removed_from_profile: string[];
  loader_mismatch: boolean;
}

export interface InstanceLogsPayload {
  log_source: string;
  lines: string[];
  crash_report?: string | null;
}

export type WorkspaceTab = "play" | "mods" | "servers" | "console";

