import { invoke } from '@tauri-apps/api/core';
import type { Profile, Backup, ChromeLaunchResult, BackupResult, RestoreResult, PerformanceRecord, Workspace, PathsInfo, LaunchArgs, BackupInspectInfo, ProxyTestResult, PortStatus, TraySettings } from '../types';

export const api = {
  // Profile operations
  getProfiles: (): Promise<Profile[]> => invoke('get_profiles'),

  getProfile: (id: string): Promise<Profile | null> => invoke('get_profile', { id }),

  /** templateMode: 'extensions' 只复制扩展与脚本；'full' 完整克隆（含登录态） */
  createProfile: (
    name: string,
    chromePath?: string,
    homepage?: string,
    iconBase64?: string,
    iconColor?: string,
    tags?: string,
    notes?: string,
    forwardPort?: number,
    disableExtensions?: boolean,
    debuggerMode?: boolean,
    debugPortConfig?: number,
    workspaceId?: string | null,
    templateId?: string,
    templateMode?: string,
    launchArgs?: LaunchArgs
  ): Promise<Profile> =>
    invoke('create_profile', {
      name,
      chromePath,
      homepage,
      iconBase64,
      iconColor,
      tags,
      notes,
      forwardPort,
      disableExtensions,
      debuggerMode,
      debugPortConfig,
      workspaceId,
      templateId,
      templateMode,
      launchArgs
    }),

  setProfileTemplate: (id: string, isTemplate: boolean): Promise<boolean> =>
    invoke('set_profile_template', { id, isTemplate }),

  getTemplateIds: (): Promise<string[]> => invoke('get_template_ids'),

  /** iconColor 传空串表示清空，回到按 id 自动取色；notes 同理 */
  updateProfile: (id: string, name?: string, chromePath?: string, homepage?: string, iconBase64?: string, iconColor?: string, tags?: string, notes?: string, forwardPort?: number | null, disableExtensions?: boolean, debuggerMode?: boolean, debugPortConfig?: number | null, launchArgs?: LaunchArgs): Promise<boolean> =>
    invoke('update_profile', { id, name, chromePath, homepage, iconBase64, iconColor, tags, notes, forwardPort, disableExtensions, debuggerMode, debugPortConfig, launchArgs }),

  deleteProfile: (id: string): Promise<boolean> => invoke('delete_profile', { id }),

  // Chrome operations
  launchChrome: (id: string, url?: string): Promise<ChromeLaunchResult> =>
    invoke('launch_chrome', { id, url }),

  /** 关闭一个配置对应的浏览器：先正常退出，没反应再强杀 */
  closeProfile: (id: string): Promise<boolean> => invoke('close_profile', { id }),

  /** 一键关闭所有运行中的配置，返回处理条数 */
  closeAllProfiles: (): Promise<number> => invoke('close_all_profiles'),

  /** 用配置里填的代理发一次请求，返回出口 IP，用来验证代理是否真的通 */
  testProxy: (proxy: string): Promise<ProxyTestResult> => invoke('test_proxy', { proxy }),

  checkPort: (port: number): Promise<PortStatus> => invoke('check_port', { port }),

  /** 从 start 开始找第一个空闲端口 */
  findFreePort: (start: number): Promise<number | null> => invoke('find_free_port', { start }),

  getTraySettings: (): Promise<TraySettings> => invoke('get_tray_settings'),

  setTraySettings: (closeToTray: boolean, autostart: boolean): Promise<TraySettings> =>
    invoke('set_tray_settings', { closeToTray, autostart }),

  // Backup operations
  backupProfile: (id: string, backupDir: string): Promise<BackupResult> =>
    invoke('backup_profile', { id, backupDir }),

  /** 批量备份：一次调用打包多个配置，后端逐个上报 backup-progress */
  backupProfiles: (ids: string[], backupDir: string): Promise<BackupResult[]> =>
    invoke('backup_profiles', { ids, backupDir }),

  /** 批量移动到同一工作区，返回实际变更条数 */
  moveProfilesToWorkspace: (ids: string[], workspaceId: string | null): Promise<number> =>
    invoke('move_profiles_to_workspace', { ids, workspaceId }),

  restoreProfile: (id: string, backupPath: string): Promise<RestoreResult> =>
    invoke('restore_profile', { id, backupPath }),

  getBackups: (id: string): Promise<Backup[]> => invoke('get_backups', { id }),

  deleteBackup: (id: string): Promise<boolean> => invoke('delete_backup', { id }),

  // Search operations
  searchProfiles: (query: string): Promise<Profile[]> => invoke('search_profiles', { query }),

  getProfilesByTag: (tag: string): Promise<Profile[]> => invoke('get_profiles_by_tag', { tag }),

  // Performance operations
  getPerformanceLogs: (id: string, limit?: number): Promise<PerformanceRecord[]> => invoke('get_performance_logs', { id, limit }),

  // Utility
  getProfileSize: (id: string): Promise<string> => invoke('get_profile_size', { id }),

  // Directory operations
  getAppDataDir: (): Promise<string> => invoke('get_app_data_dir'),

  openProfileDirectory: (profileDataDir: string) =>
    invoke<void>('open_profile_directory', { profileDataDir }),

  unlockProfile: (id: string) => invoke<void>('unlock_profile', { id }),

  // Workspace operations
  getWorkspaces: (): Promise<Workspace[]> => invoke('get_workspaces'),

  createWorkspace: (name: string, icon?: string, color?: string, description?: string): Promise<Workspace> =>
    invoke('create_workspace', { name, icon, color, description }),

  updateWorkspace: (id: string, name?: string, icon?: string, color?: string, description?: string): Promise<boolean> =>
    invoke('update_workspace', { id, name, icon, color, description }),

  deleteWorkspace: (id: string): Promise<boolean> => invoke('delete_workspace', { id }),

  getProfilesByWorkspace: (workspaceId: string): Promise<Profile[]> =>
    invoke('get_profiles_by_workspace', { workspaceId }),

  moveProfileToWorkspace: (id: string, workspaceId?: string | null): Promise<boolean> =>
    invoke('move_profile_to_workspace', { id, workspaceId }),

  backupWorkspace: (workspaceId: string, backupDir: string): Promise<BackupResult[]> =>
    invoke('backup_workspace', { workspaceId, backupDir }),

  // 缓存清理与体积统计
  clearProfileCache: (id: string): Promise<number> =>
    invoke('clear_profile_cache', { id }),

  /** 批量清理缓存，运行中的配置会被跳过 */
  clearProfilesCache: (ids: string[]): Promise<{ freed_bytes: number; cleared: number; skipped: number }> =>
    invoke('clear_profiles_cache', { ids }),

  getProfileSizes: (): Promise<[string, number][]> => invoke('get_profile_sizes'),

  /** 整体重写排序，数组下标即新的顺序 */
  reorderProfiles: (ids: string[]): Promise<void> => invoke('reorder_profiles', { ids }),

  // 数据目录设置
  getPaths: (): Promise<PathsInfo> => invoke('get_paths'),

  getBackupDir: async (): Promise<string> => {
    const info = await invoke<PathsInfo>('get_paths');
    return info.backup_dir;
  },

  setDataDir: (path: string): Promise<PathsInfo> => invoke('set_data_dir', { path }),

  setBackupDir: (path: string): Promise<PathsInfo> => invoke('set_backup_dir', { path }),

  /** 每个配置保留的备份数量，0 = 不限制 */
  setBackupRetention: (keep: number): Promise<PathsInfo> =>
    invoke('set_backup_retention', { keep }),

  /** 手动记录当前窗口大小与位置 */
  saveWindowState: (): Promise<void> => invoke('save_window_state'),

  /** 读入备份包元信息，用于导入前的确认 */
  inspectBackup: (backupPath: string): Promise<BackupInspectInfo> =>
    invoke('inspect_backup', { backupPath }),

  /** 从备份包导入：按包里的软件配置还原配置记录，再解压浏览器数据 */
  importProfileBackup: (backupPath: string, workspaceId?: string | null): Promise<Profile> =>
    invoke('import_profile_backup', { backupPath, workspaceId }),

  /** 无边框窗口的操作：minimize / maximize / close / drag */
  windowAction: (action: 'minimize' | 'maximize' | 'close' | 'drag'): Promise<void> =>
    invoke('window_action', { action }),
};
