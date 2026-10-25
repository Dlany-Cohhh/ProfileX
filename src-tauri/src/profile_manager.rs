use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;
use zip::ZipArchive;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use crate::db::{LaunchArgs, Profile, Workspace};

/// 备份包内存放「软件配置」的目录。恢复浏览器数据时会被跳过，
/// 免得把不属于浏览器的文件写进配置目录
pub const BACKUP_META_FILE: &str = "_spaceshift/meta.json";

pub const BACKUP_META_FORMAT: &str = "spaceshift-profile-backup";
pub const BACKUP_META_VERSION: u32 = 1;

/// 配置在软件里的设置：名字、图标、标签、端口、启动参数、模板标记等
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMeta {
    pub name: String,
    pub chrome_path: Option<String>,
    pub homepage: Option<String>,
    pub icon_base64: Option<String>,
    pub icon_color: Option<String>,
    pub tags: Option<String>,
    pub notes: Option<String>,
    pub forward_port: Option<u16>,
    pub disable_extensions: bool,
    pub debugger_mode: bool,
    pub debug_port_config: Option<u16>,
    pub launch_args: Option<LaunchArgs>,
    pub is_template: bool,
    pub sort_order: Option<i64>,
}

/// 配置所属工作区，导入时按 id 还原，没有就按名字建一个
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkspaceMeta {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
}

/// 整包元信息，写在备份包根目录的 _spaceshift/meta.json
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupMeta {
    pub format: String,
    pub version: u32,
    pub exported_at: String,
    pub profile_id: String,
    pub profile: ProfileMeta,
    pub workspace: Option<WorkspaceMeta>,
}

/// 组装要写进备份包的软件配置
pub fn build_backup_meta(profile: &Profile, workspace: Option<&Workspace>) -> BackupMeta {
    BackupMeta {
        format: BACKUP_META_FORMAT.to_string(),
        version: BACKUP_META_VERSION,
        exported_at: Utc::now().to_rfc3339(),
        profile_id: profile.id.clone(),
        profile: ProfileMeta {
            name: profile.name.clone(),
            chrome_path: profile.chrome_path.clone(),
            homepage: profile.homepage.clone(),
            icon_base64: profile.icon_base64.clone(),
            icon_color: profile.icon_color.clone(),
            tags: profile.tags.clone(),
            notes: profile.notes.clone(),
            forward_port: profile.forward_port,
            disable_extensions: profile.disable_extensions,
            debugger_mode: profile.debugger_mode,
            debug_port_config: profile.debug_port_config,
            launch_args: Some(profile.launch_args.clone()),
            is_template: profile.is_template,
            sort_order: profile.sort_order,
        },
        workspace: workspace.map(|ws| WorkspaceMeta {
            id: ws.id.clone(),
            name: ws.name.clone(),
            icon: ws.icon.clone(),
            color: ws.color.clone(),
            description: ws.description.clone(),
        }),
    }
}

/// 读取备份包里的软件配置；老包没有这个文件就返回 None
pub fn read_backup_meta(backup_path: &Path) -> Option<BackupMeta> {
    let file = fs::File::open(backup_path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;

    let mut entry = archive.by_name(BACKUP_META_FILE).ok()?;
    let text = io::read_to_string(&mut entry).ok()?;

    let meta: BackupMeta = serde_json::from_str(&text).ok()?;

    if meta.format != BACKUP_META_FORMAT {
        return None;
    }

    Some(meta)
}

/// 从文件名推断配置名：去掉末尾的 _20261006_093844 时间戳
pub fn profile_name_from_backup_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "导入的配置".to_string());

    // 名字_20261006_093844
    let bytes: Vec<(usize, char)> = stem.char_indices().collect();
    let Some((underscore, _)) = bytes.iter().rev().nth(15).copied() else {
        return stem;
    };

    let tail = &stem[underscore + 1..];
    let is_timestamp =
        tail.len() == 15 && tail.chars().all(|c| c.is_ascii_digit() || c == '_');

    if is_timestamp {
        stem[..underscore].to_string()
    } else {
        stem
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupResult {
    pub success: bool,
    pub backup_path: Option<String>,
    pub size_bytes: i64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreResult {
    pub success: bool,
    pub error: Option<String>,
}

/// 模板「仅扩展与脚本」模式要复制的条目（位于浏览器 profile 目录内，如 Default/）
/// 扩展本体在 Extensions，Tampermonkey 之类的脚本存在 Local/Sync Extension Settings，
/// Preferences / Secure Preferences 记录扩展的启用状态与权限，缺了扩展不会被加载
const TEMPLATE_EXTENSION_ITEMS: &[&str] = &[
    "Extensions",
    "Extension Rules",
    "Extension State",
    "Local Extension Settings",
    "Sync Extension Settings",
    "Preferences",
    "Secure Preferences",
];

/// 模板「完整克隆」模式要排除的易失/缓存条目
const TEMPLATE_SKIP_ITEMS: &[&str] = &[
    "Cache",
    "CacheStorage",
    "Code Cache",
    "GPUCache",
    "GrShaderCache",
    "ShaderCache",
    "Sessions",
    "Sessions_Encrypted",
    "Session Storage",
    "Service Worker",
    "Crashpad",
    "BrowserMetrics",
    "BrowserMetrics-spare.pma",
    "VideoDecodeStats",
    "optimization_guide_hint_cache_store",
];

/// 含 Preferences 的目录才是浏览器 profile 目录（Default、Profile 1 …）
fn is_browser_profile_dir(dir: &Path) -> bool {
    dir.join("Preferences").exists()
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(&entry.path(), &target)?;
        }
    }

    Ok(())
}

/// 可由浏览器重新生成、无需进备份包的缓存目录（按目录名匹配）
const CACHE_DIRS: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "GrShaderCache",
    "ShaderCache",
    "CacheStorage",
    "Service Worker",
    "Application Cache",
    "component_crx_cache",
    "extensions_crx_cache",
    "ProvenanceData",
    "BrowserMetrics",
    "BrowserMetrics-spare.pma",
    "WidevineCdm",
    "Subresource Filter",
    "OptimizationHints",
];

/// 执行「清理缓存」时真正删除的目录，Cookie / 登录态 / 扩展配置均不在此列
const CLEARABLE_CACHE_DIRS: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "GrShaderCache",
    "ShaderCache",
    "CacheStorage",
    "Service Worker",
    "Application Cache",
    "component_crx_cache",
    "extensions_crx_cache",
    "ProvenanceData",
    "BrowserMetrics",
    "BrowserMetrics-spare.pma",
    "WidevineCdm",
    "Subresource Filter",
    "OptimizationHints",
];

fn is_cache_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| CACHE_DIRS.iter().any(|d| n.eq_ignore_ascii_case(d)))
        .unwrap_or(false)
}

pub struct ProfileManager;

impl ProfileManager {
    /// 清理缓存，返回释放的字节数。不会触碰 Cookie、登录态、扩展配置
    pub fn clear_profile_cache(profile_dir: &PathBuf) -> u64 {
        let mut freed: u64 = 0;

        // contents_first：自底向上，先删子目录再删父目录，避免遍历被自己的删除打断
        for entry in WalkDir::new(profile_dir).contents_first(true) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            if !entry.file_type().is_dir() {
                continue;
            }

            let matched = entry
                .file_name()
                .to_str()
                .map(|n| CLEARABLE_CACHE_DIRS.iter().any(|d| n.eq_ignore_ascii_case(d)))
                .unwrap_or(false);

            if !matched {
                continue;
            }

            freed += Self::get_directory_size(&entry.path().to_path_buf()).unwrap_or(0);

            // 浏览器仍占用时可能删除失败，忽略即可，下次再清理
            let _ = fs::remove_dir_all(entry.path());
        }

        freed
    }

    /// 把模板配置的浏览器数据套用到新配置上，返回复制的条目数。
    /// full = true 时完整克隆（含登录态与站点数据），否则只复制扩展与脚本。
    pub fn apply_template(template_dir: &Path, target_dir: &Path, full: bool) -> io::Result<usize> {
        let mut copied = 0usize;

        for entry in fs::read_dir(template_dir)?.flatten() {
            if entry.file_type()?.is_dir() == false {
                continue;
            }

            // 根目录下混杂着大量组件目录，只处理真正的浏览器 profile 目录
            if !is_browser_profile_dir(&entry.path()) {
                continue;
            }

            let dir_name = entry.file_name();
            let dir_name = dir_name.to_string_lossy().to_string();
            let target_profile_dir = target_dir.join(&dir_name);
            fs::create_dir_all(&target_profile_dir)?;

            for item in fs::read_dir(entry.path())?.flatten() {
                let item_name = item.file_name();
                let item_name = item_name.to_string_lossy().to_string();

                if full {
                    if TEMPLATE_SKIP_ITEMS.contains(&item_name.as_str()) {
                        continue;
                    }
                } else if !TEMPLATE_EXTENSION_ITEMS.contains(&item_name.as_str()) {
                    continue;
                }

                let item_target = target_profile_dir.join(&item_name);

                if item.file_type()?.is_dir() {
                    copy_dir_recursive(&item.path(), &item_target)?;
                } else {
                    fs::copy(&item.path(), &item_target)?;
                }

                copied += 1;
            }
        }

        Ok(copied)
    }

    pub fn create_profile_directory(base_dir: &PathBuf, _profile_name: &str) -> io::Result<PathBuf> {
        let uuid = uuid::Uuid::new_v4().to_string().replace('-', "");
        let profile_dir = base_dir.join(&uuid);
        fs::create_dir_all(&profile_dir)?;
        
        Ok(profile_dir)
    }

    pub fn delete_profile_directory(dir_path: &PathBuf) -> io::Result<()> {
        if dir_path.exists() {
            fs::remove_dir_all(dir_path)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn rename_profile_directory(old_path: &PathBuf, new_name: &str) -> io::Result<PathBuf> {
        let parent = old_path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid path")
        })?;
        
        let sanitized_name = new_name
            .replace(|c: char| !c.is_alphanumeric() && c != ' ' && c != '-', "_")
            .replace(' ', "_");
        
        let new_path = parent.join(&sanitized_name);
        
        if old_path.exists() {
            fs::rename(old_path, &new_path)?;
        }
        
        Ok(new_path)
    }

    /// meta_json 为软件配置（profile + workspace）的 JSON，写进包里供导入时原样恢复
    pub fn backup_profile(profile_dir: &PathBuf, backup_dir: &PathBuf, profile_name: &str, meta_json: Option<&str>) -> BackupResult {
        if !profile_dir.exists() {
            return BackupResult {
                success: false,
                backup_path: None,
                size_bytes: 0,
                error: Some(format!("Profile directory does not exist: {:?}", profile_dir)),
            };
        }

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("{}_{}.zip", profile_name.replace(' ', "_"), timestamp);
        let backup_path = backup_dir.join(&backup_filename);

        if let Some(parent) = backup_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return BackupResult {
                    success: false,
                    backup_path: None,
                    size_bytes: 0,
                    error: Some(format!("Failed to create backup directory: {}", e)),
                };
            }
        }

        let file = match fs::File::create(&backup_path) {
            Ok(f) => f,
            Err(e) => {
                return BackupResult {
                    success: false,
                    backup_path: None,
                    size_bytes: 0,
                    error: Some(format!("Failed to create backup file: {}", e)),
                };
            }
        };

        let buf_writer = io::BufWriter::new(file);
        let mut zip = ZipWriter::new(buf_writer);
        // 用最快压缩：备份包只是本地归档，压缩率高 15% 换不来 2~3 倍的耗时
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(1))
            .unix_permissions(0o755);

        // 软件配置写在最前面，读包时一眼能看到这份备份属于谁
        if let Some(json) = meta_json {
            if zip.start_file(BACKUP_META_FILE, options).is_err() {
                return BackupResult {
                    success: false,
                    backup_path: None,
                    size_bytes: 0,
                    error: Some("无法写入软件配置".to_string()),
                };
            }

            if io::Write::write_all(&mut zip, json.as_bytes()).is_err() {
                return BackupResult {
                    success: false,
                    backup_path: None,
                    size_bytes: 0,
                    error: Some("无法写入软件配置".to_string()),
                };
            }
        }

        let mut total_size: i64 = 0;

        for entry in WalkDir::new(profile_dir) {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    let name = path.strip_prefix(profile_dir).unwrap_or(path);
                    let name_str = name.to_string_lossy();

                    // 跳过缓存目录：它们可从网络/本地重新生成，打进去只会让备份包膨胀
                    if path.is_dir() && is_cache_dir(path) {
                        continue;
                    }

                    if path.is_file() {
                        let mut f = match fs::File::open(path) {
                            Ok(f) => f,
                            Err(_) => continue, // Skip files that can't be opened (e.g. in use)
                        };

                        let metadata = match entry.metadata() {
                            Ok(m) => m,
                            Err(_) => continue,
                        };

                        total_size += metadata.len() as i64;
                        
                        if let Err(e) = zip.start_file(name_str.as_ref(), options) {
                            return BackupResult {
                                success: false,
                                backup_path: None,
                                size_bytes: 0,
                                error: Some(format!("Failed to add file to zip: {}", e)),
                            };
                        }

                        if let Err(e) = io::copy(&mut f, &mut zip) {
                            return BackupResult {
                                success: false,
                                backup_path: None,
                                size_bytes: 0,
                                error: Some(format!("Failed to write file contents: {}", e)),
                            };
                        }
                    } else if path.is_dir() && !name_str.is_empty() {
                        let dir_name = format!("{}/", name_str);
                        if let Err(e) = zip.add_directory(&dir_name, options) {
                            return BackupResult {
                                success: false,
                                backup_path: None,
                                size_bytes: 0,
                                error: Some(format!("Failed to add directory to zip: {}", e)),
                            };
                        }
                    }
                }
                Err(e) => {
                    return BackupResult {
                        success: false,
                        backup_path: None,
                        size_bytes: 0,
                        error: Some(format!("Failed to walk directory: {}", e)),
                    };
                }
            }
        }

        match zip.finish() {
            Ok(_) => BackupResult {
                success: true,
                backup_path: Some(backup_path.to_string_lossy().to_string()),
                size_bytes: total_size,
                error: None,
            },
            Err(e) => BackupResult {
                success: false,
                backup_path: None,
                size_bytes: 0,
                error: Some(format!("Failed to finalize zip: {}", e)),
            },
        }
    }

    pub fn restore_profile(backup_path: &PathBuf, target_dir: &PathBuf) -> RestoreResult {
        if !backup_path.exists() {
            return RestoreResult {
                success: false,
                error: Some(format!("Backup file does not exist: {:?}", backup_path)),
            };
        }

        let file = match fs::File::open(backup_path) {
            Ok(f) => f,
            Err(e) => {
                return RestoreResult {
                    success: false,
                    error: Some(format!("Failed to open backup file: {}", e)),
                };
            }
        };

        let mut archive = match zip::ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                return RestoreResult {
                    success: false,
                    error: Some(format!("Failed to read zip archive: {}", e)),
                };
            }
        };

        if let Err(e) = fs::create_dir_all(target_dir) {
            return RestoreResult {
                success: false,
                error: Some(format!("Failed to create target directory: {}", e)),
            };
        }

        for i in 0..archive.len() {
            let mut file = match archive.by_index(i) {
                Ok(f) => f,
                Err(e) => {
                    return RestoreResult {
                        success: false,
                        error: Some(format!("Failed to read file from archive: {}", e)),
                    };
                }
            };

            // 软件配置不属于浏览器数据，不解压进配置目录
            if file.name().starts_with("_spaceshift/") {
                continue;
            }

            let outpath = target_dir.join(file.name());

            if file.name().ends_with('/') {
                if let Err(e) = fs::create_dir_all(&outpath) {
                    return RestoreResult {
                        success: false,
                        error: Some(format!("Failed to create directory: {}", e)),
                    };
                }
            } else {
                if let Some(parent) = outpath.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        return RestoreResult {
                            success: false,
                            error: Some(format!("Failed to create parent directory: {}", e)),
                        };
                    }
                }

                let mut outfile = match fs::File::create(&outpath) {
                    Ok(f) => f,
                    Err(e) => {
                        return RestoreResult {
                            success: false,
                            error: Some(format!("Failed to create output file: {}", e)),
                        };
                    }
                };

                if let Err(e) = io::copy(&mut file, &mut outfile) {
                    return RestoreResult {
                        success: false,
                        error: Some(format!("Failed to write file: {}", e)),
                    };
                }
            }
        }

        RestoreResult {
            success: true,
            error: None,
        }
    }

    pub fn get_directory_size(dir: &PathBuf) -> io::Result<u64> {
        let mut total_size: u64 = 0;

        for entry in WalkDir::new(dir) {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        total_size += metadata.len();
                    }
                }
            }
        }

        Ok(total_size)
    }

    pub fn format_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile(name: &str) -> Profile {
        Profile {
            id: "profile-1".to_string(),
            name: name.to_string(),
            data_dir_path: String::new(),
            chrome_path: None,
            homepage: Some("https://example.com".to_string()),
            icon_path: None,
            icon_base64: None,
            icon_color: None,
            tags: Some("工作,测试".to_string()),
            notes: Some("备注：外贸主账号".to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            last_opened_at: None,
            is_running: false,
            pid: None,
            debug_port: None,
            forward_port: Some(9222),
            disable_extensions: true,
            debugger_mode: true,
            debug_port_config: Some(9333),
            workspace_id: Some("ws-1".to_string()),
            launch_args: LaunchArgs {
                proxy: Some("http://127.0.0.1:7890".to_string()),
                ..LaunchArgs::default()
            },
            is_template: true,
            sort_order: Some(7),
        }
    }

    fn sample_workspace() -> Workspace {
        Workspace {
            id: "ws-1".to_string(),
            name: "日常".to_string(),
            icon: Some("📁".to_string()),
            color: Some("#409eff".to_string()),
            description: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    /// 备份包要带上软件配置，且这些配置不能被解压进浏览器数据目录
    #[test]
    fn backup_keeps_app_settings_and_restore_skips_them() {
        let root = std::env::temp_dir().join(format!("spaceshift-meta-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);

        let profile_dir = root.join("profile");
        fs::create_dir_all(profile_dir.join("Default")).unwrap();
        fs::write(profile_dir.join("Default").join("Preferences"), "{\"a\":1}").unwrap();

        let backup_dir = root.join("backups");
        let meta = build_backup_meta(&sample_profile("测试 配置"), Some(&sample_workspace()));
        let json = serde_json::to_string(&meta).unwrap();

        let result = ProfileManager::backup_profile(&profile_dir, &backup_dir, "测试 配置", Some(&json));
        assert!(result.success, "{:?}", result.error);

        let backup_path = PathBuf::from(result.backup_path.unwrap());

        // 软件配置能原样读回来
        let read = read_backup_meta(&backup_path).expect("备份包里应有软件配置");
        assert_eq!(read.profile.name, "测试 配置");
        assert_eq!(read.profile.tags.as_deref(), Some("工作,测试"));
        assert_eq!(read.profile.forward_port, Some(9222));
        assert!(read.profile.disable_extensions);
        assert!(read.profile.is_template);
        assert_eq!(read.profile.sort_order, Some(7));
        assert_eq!(read.profile.launch_args.as_ref().unwrap().proxy.as_deref(), Some("http://127.0.0.1:7890"));
        assert_eq!(read.workspace.as_ref().unwrap().name, "日常");
        assert_eq!(read.profile_id, "profile-1");

        // 浏览器数据恢复，配置目录不出现 _spaceshift
        let restored = root.join("restored");
        let restore = ProfileManager::restore_profile(&backup_path, &restored);
        assert!(restore.success, "{:?}", restore.error);
        assert!(restored.join("Default").join("Preferences").exists());
        assert!(!restored.join("_spaceshift").exists());

        let _ = fs::remove_dir_all(&root);
    }

    /// 旧版备份包没有 meta：读出来是 None，配置名靠文件名推断
    #[test]
    fn legacy_backup_has_no_meta() {
        let root = std::env::temp_dir().join(format!("spaceshift-legacy-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);

        let profile_dir = root.join("profile");
        fs::create_dir_all(&profile_dir).unwrap();
        fs::write(profile_dir.join("data.txt"), "x").unwrap();

        let backup_dir = root.join("backups");
        let result = ProfileManager::backup_profile(&profile_dir, &backup_dir, "旧配置", None);
        let backup_path = PathBuf::from(result.backup_path.unwrap());

        assert!(read_backup_meta(&backup_path).is_none());
        assert_eq!(profile_name_from_backup_path(&backup_path), "旧配置");

        let _ = fs::remove_dir_all(&root);
    }
}
