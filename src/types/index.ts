/** 浏览器启动参数，透传给 Chrome 命令行 */
export interface LaunchArgs {
  proxy?: string;
  userAgent?: string;
  windowSize?: string;
  windowPosition?: string;
  language?: string;
  extraArgs?: string;
}

export interface Profile {
  id: string;
  name: string;
  data_dir_path: string;
  chrome_path?: string;
  homepage?: string;
  icon_path?: string;
  icon_base64?: string;
  tags?: string;
  created_at: string;
  updated_at: string;
  last_opened_at?: string;
  is_running: boolean;
  pid?: number;
  debug_port?: number;
  forward_port?: number;
  disable_extensions: boolean;
  debugger_mode: boolean;
  debug_port_config?: number;
  workspace_id?: string | null;
  launch_args: LaunchArgs;
}

export interface PathsInfo {
  app_data_dir: string;
  profiles_dir: string;
  backup_dir: string;
  using_custom_data_dir: boolean;
  using_custom_backup_dir: boolean;
  backup_retention: number;
}

export interface Workspace {
  id: string;
  name: string;
  icon?: string | null;
  color?: string | null;
  description?: string | null;
  created_at: string;
  updated_at: string;
}

export interface Backup {
  id: string;
  profile_id: string;
  backup_path: string;
  created_at: string;
  size_bytes: number;
}

export interface ChromeLaunchResult {
  success: boolean;
  pid?: number;
  debug_port?: number;
  error?: string;
}

export interface BackupResult {
  success: boolean;
  backup_path?: string;
  size_bytes: number;
  error?: string;
}

export interface RestoreResult {
  success: boolean;
  error?: string;
}

export type ViewMode = 'grid' | 'list';

export interface PerformanceRecord {
  id: string;
  profile_id: string;
  launch_duration_ms: number;
  spawn_duration_ms: number;
  dns_duration_ms?: number;
  tcp_duration_ms?: number;
  dom_ready_ms?: number;
  page_load_ms?: number;
  created_at: string;
}
