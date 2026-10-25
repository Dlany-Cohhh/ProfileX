use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

mod db;
mod chrome;
mod profile_manager;
mod network_optimizer;
mod performance_monitor;
mod browser_monitor;
mod error;
mod cdp;

use db::{Database, Profile, Backup, PerformanceRecord, Workspace, LaunchArgs};
use chrome::{ChromeManager, ChromeLaunchResult};
use profile_manager::{ProfileManager, BackupResult, RestoreResult};
use browser_monitor::BrowserMonitor;
use error::{AppError, AppResult};
use std::sync::Arc;
use socket2::{SockRef, TcpKeepalive};
use std::time::Duration;

struct AppState {
    db: Arc<Mutex<Database>>,
    chrome_manager: ChromeManager,
}

/// 可迁移的数据目录配置，存在 app_data_dir/config.json
#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
#[serde(default)]
struct AppConfig {
    data_dir: Option<String>,
    backup_dir: Option<String>,
    /// 每个配置保留的备份数量，0 = 不限制
    backup_retention: Option<usize>,
    /// 窗口几何记忆
    window: Option<WindowState>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone, Copy)]
struct WindowState {
    width: f64,
    height: f64,
    x: f64,
    y: f64,
}

fn backup_retention_of(app: &AppHandle) -> usize {
    load_config(app).backup_retention.unwrap_or(0)
}

/// 删除备份文件。记录里的路径可能是搬迁前的旧位置，
/// 这时改到当前备份目录下按同名文件删，避免留下删不掉的孤儿文件
fn remove_backup_file(app: &AppHandle, path: &str) {
    let target = PathBuf::from(path);
    if target.exists() {
        let _ = fs::remove_file(target);
        return;
    }

    let Some(name) = target.file_name() else {
        return;
    };

    if let Ok(backup_dir) = backup_dir_of(app) {
        let moved = backup_dir.join(name);
        if moved.exists() {
            let _ = fs::remove_file(moved);
        }
    }
}

/// 按保留策略清理某个配置的旧备份
fn prune_backups_for(app: &AppHandle, db: &Database, profile_id: &str) {
    let keep = backup_retention_of(app);
    if keep == 0 {
        return;
    }

    let Ok(stale) = db.prune_backups(profile_id, keep) else {
        return;
    };

    for path in stale {
        remove_backup_file(app, &path);
    }
}

#[derive(serde::Serialize)]
struct PathsInfo {
    app_data_dir: String,
    profiles_dir: String,
    backup_dir: String,
    using_custom_data_dir: bool,
    using_custom_backup_dir: bool,
    backup_retention: usize,
}

fn app_data_dir_of(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app.path().app_data_dir()?)
}

fn load_config(app: &AppHandle) -> AppConfig {
    let Ok(dir) = app_data_dir_of(app) else {
        return AppConfig::default();
    };

    match fs::read_to_string(dir.join("config.json")) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

fn save_config(app: &AppHandle, config: &AppConfig) -> AppResult<()> {
    let dir = app_data_dir_of(app)?;
    fs::create_dir_all(&dir)?;

    let content =
        serde_json::to_string_pretty(config).map_err(|e| AppError::Other(e.to_string()))?;
    fs::write(dir.join("config.json"), content)?;

    Ok(())
}

fn profiles_dir_of(app: &AppHandle) -> AppResult<PathBuf> {
    let base = app_data_dir_of(app)?;

    match load_config(app).data_dir {
        Some(dir) if !dir.trim().is_empty() => Ok(PathBuf::from(dir)),
        _ => Ok(base.join("profiles")),
    }
}

fn backup_dir_of(app: &AppHandle) -> AppResult<PathBuf> {
    match load_config(app).backup_dir {
        Some(dir) if !dir.trim().is_empty() => Ok(PathBuf::from(dir)),
        _ => {
            let home = dirs::home_dir()
                .ok_or_else(|| AppError::InvalidPath("无法定位用户目录".to_string()))?;
            Ok(home.join("SpaceShift").join("Backups"))
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
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

/// 备份进度事件，前端用它显示「正在备份 3/8」
#[derive(serde::Serialize, Clone)]
struct BackupProgress {
    current: usize,
    total: usize,
    name: String,
}

/// 搬迁进度事件，前端用它画进度条
#[derive(serde::Serialize, Clone)]
struct MigrationProgress {
    current: usize,
    total: usize,
    label: String,
}

/// 搬迁目录内容，返回 (成功数, 跳过数)。跨盘时 rename 会失败，退化为复制后删除
fn move_dir_contents_with_progress(
    src: &Path,
    dst: &Path,
    mut on_progress: impl FnMut(usize, usize, String),
) -> (usize, usize) {
    let entries: Vec<PathBuf> = match fs::read_dir(src) {
        Ok(entries) => entries.flatten().map(|entry| entry.path()).collect(),
        Err(_) => return (0, 0),
    };

    let total = entries.len();
    let mut moved = 0usize;
    let mut skipped = 0usize;

    for (index, path) in entries.into_iter().enumerate() {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // 先上报进度，再开工，这样界面能立刻动起来
        on_progress(index, total, name.clone());

        let target = dst.join(&name);

        // 目标已存在同名目录则不覆盖，避免误伤已有数据
        if target.exists() {
            skipped += 1;
            continue;
        }

        if fs::rename(&path, &target).is_ok() {
            moved += 1;
            continue;
        }

        if copy_dir_recursive(&path, &target).is_ok() {
            let _ = fs::remove_dir_all(&path);
            moved += 1;
        } else {
            skipped += 1;
        }
    }

    (moved, skipped)
}

/// 判断是否已有配置在运行，搬迁前必须先全部关闭
fn running_profile_count(state: &State<AppState>) -> AppResult<usize> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.get_all_profiles()?.iter().filter(|p| p.is_running).count())
}

#[tauri::command]
fn get_profiles(state: State<AppState>) -> AppResult<Vec<Profile>> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.get_all_profiles()?)
}

#[tauri::command]
fn get_profile(id: String, state: State<AppState>) -> AppResult<Option<Profile>> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.get_profile_by_id(&id)?)
}

#[tauri::command]
fn create_profile(
    name: String,
    chrome_path: Option<String>,
    homepage: Option<String>,
    icon_base64: Option<String>,
    tags: Option<String>,
    forward_port: Option<u16>,
    disable_extensions: Option<bool>,
    debugger_mode: Option<bool>,
    debug_port_config: Option<u16>,
    workspace_id: Option<String>,
    template_id: Option<String>,
    template_mode: Option<String>,
    launch_args: Option<LaunchArgs>,
    app_handle: AppHandle,
    state: State<AppState>,
) -> AppResult<Profile> {
    let profiles_dir = profiles_dir_of(&app_handle)?;

    std::fs::create_dir_all(&profiles_dir)?;

    // 先取出模板并校验，避免校验失败时留下一个空目录
    let template = match template_id.as_deref() {
        Some(id) if !id.is_empty() => {
            let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
            db.get_profile_by_id(id)?
        }
        _ => None,
    };

    if let Some(running) = template.as_ref().filter(|t| t.is_running) {
        return Err(AppError::Other(format!(
            "模板配置「{}」正在运行，请先关闭它再套用",
            running.name
        )));
    }

    let profile_dir = ProfileManager::create_profile_directory(&profiles_dir, &name)?;

    // 套用模板：把模板配置的浏览器数据复制进新目录，扩展与脚本才会跟着过来
    let applied = match template {
        Some(template) => {
            let src = PathBuf::from(&template.data_dir_path);

            if src.exists() {
                ProfileManager::apply_template(
                    &src,
                    &profile_dir,
                    template_mode.as_deref() == Some("full"),
                )?;
                true
            } else {
                false
            }
        }
        None => false,
    };

    // 套用了模板就必须允许加载扩展，否则复制过去的扩展不会生效
    let disable_extensions = if applied {
        false
    } else {
        disable_extensions.unwrap_or(true)
    };

    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.create_profile(&name, &profile_dir.to_string_lossy(), chrome_path.as_deref(), homepage.as_deref(), icon_base64.as_deref(), tags.as_deref(), forward_port, disable_extensions, debugger_mode.unwrap_or(true), debug_port_config, workspace_id.as_deref(), launch_args.as_ref())?)
}

#[tauri::command]
fn set_profile_template(
    id: String,
    is_template: bool,
    state: State<AppState>,
) -> AppResult<bool> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.set_profile_template(&id, is_template)?)
}

#[tauri::command]
fn get_template_ids(state: State<AppState>) -> AppResult<Vec<String>> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.get_template_ids()?)
}

#[tauri::command]
fn update_profile(
    id: String,
    name: Option<String>,
    chrome_path: Option<String>,
    homepage: Option<String>,
    icon_base64: Option<String>,
    tags: Option<String>,
    forward_port: Option<u16>,
    disable_extensions: Option<bool>,
    debugger_mode: Option<bool>,
    debug_port_config: Option<u16>,
    launch_args: Option<LaunchArgs>,
    state: State<AppState>,
) -> AppResult<bool> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.update_profile(&id, name.as_deref(), chrome_path.as_deref(), homepage.as_deref(), icon_base64.as_deref(), tags.as_deref(), forward_port, disable_extensions, debugger_mode, debug_port_config, launch_args.as_ref())?)
}

/// 批量移动到同一工作区：一次调用完成，避免前端逐个往返
#[tauri::command]
fn move_profiles_to_workspace(
    ids: Vec<String>,
    workspace_id: Option<String>,
    state: State<AppState>,
) -> AppResult<usize> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.move_profiles_to_workspace(&ids, workspace_id.as_deref())?)
}

#[tauri::command]
fn delete_profile(id: String, state: State<AppState>) -> AppResult<bool> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    
    if let Ok(Some(profile)) = db.get_profile_by_id(&id) {
        let profile_dir = PathBuf::from(&profile.data_dir_path);
        ProfileManager::delete_profile_directory(&profile_dir)?;
    }
    
    Ok(db.delete_profile(&id)?)
}

#[tauri::command]
fn launch_chrome(
    id: String,
    url: Option<String>,
    state: State<AppState>,
) -> AppResult<ChromeLaunchResult> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;

    let profile = db.get_profile_by_id(&id)?
        .ok_or(AppError::ProfileNotFound)?;

    let profile_dir = PathBuf::from(&profile.data_dir_path);

    // Determine debug port based on debugger_mode setting
    let debug_port = if profile.debugger_mode {
        if let Some(configured) = profile.debug_port_config {
            // Use user-configured port
            Some(configured)
        } else {
            // Auto-allocate a free port for CDP (local only)
            std::net::TcpListener::bind("127.0.0.1:0")
                .ok()
                .and_then(|l| l.local_addr().ok())
                .map(|a| a.port())
        }
    } else {
        None
    };

    // Use the provided URL if available, otherwise use the profile's homepage
    let launch_url = url.as_deref().or(profile.homepage.as_deref());

    let start_inst = std::time::Instant::now();
    let result = state.chrome_manager.launch_chrome(
        &id,
        &profile_dir,
        profile.chrome_path.as_deref(),
        launch_url,
        debug_port,
        profile.disable_extensions,
        &profile.launch_args,
    );

    if result.success {
        db.update_profile_status(&id, true, result.pid.map(|p| p as i32), debug_port)?;
        
        // Setup forwarding if configured
        if let (Some(local_port), Some(fwd_port)) = (debug_port, profile.forward_port) {
            let profile_id = id.clone();
            tauri::async_runtime::spawn(async move {
                println!("[Lib] Starting CDP forwarder: 0.0.0.0:{} -> 127.0.0.1:{}", fwd_port, local_port);
                if let Ok(listener) = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", fwd_port)).await {
                    loop {
                        // Check if browser is still running (very basic check, could be improved)
                        // For now, we just accept connections as long as the listener is alive
                        match listener.accept().await {
                            Ok((mut client, _addr)) => {
                                let _ = client.set_nodelay(true);
                                
                                // Set keepalive for client (inbound)
                                let sock_ref = SockRef::from(&client);
                                let keepalive = TcpKeepalive::new().with_time(Duration::from_secs(60));
                                let _ = sock_ref.set_tcp_keepalive(&keepalive);

                                tokio::spawn(async move {
                                    if let Ok(mut server) = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", local_port)).await {
                                        let _ = server.set_nodelay(true);
                                        
                                        // Set keepalive for server (outbound to Chrome)
                                        let sock_ref = SockRef::from(&server);
                                        let keepalive = TcpKeepalive::new().with_time(Duration::from_secs(60));
                                        let _ = sock_ref.set_tcp_keepalive(&keepalive);

                                        if let Err(_e) = tokio::io::copy_bidirectional(&mut client, &mut server).await {
                                            // Handle error if needed
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                println!("[Lib] Forwarder accept error: {}", e);
                                break;
                            }
                        }
                    }
                } else {
                    println!("[Lib] Failed to bind forwarder port {}", fwd_port);
                }
                println!("[Lib] CDP forwarder stopped for {}", profile_id);
            });
        }
        
        // Start async monitoring task using debug_port
        if let Some(port) = debug_port {
            let db_clone = state.db.clone();
            let profile_id = id.clone();
            let spawn_ms = result.spawn_duration_ms;

            tauri::async_runtime::spawn(async move {
                println!("[Lib] Monitoring performance for port: {}", port);
                let client = reqwest::Client::new();
                let url = format!("http://127.0.0.1:{}/json/version", port);
                
                let mut ready_ms = None;
                // Poll for up to 10 seconds
                for _ in 0..20 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    if let Ok(resp) = client.get(&url).send().await {
                        if resp.status().is_success() {
                            ready_ms = Some(start_inst.elapsed().as_millis() as u64);
                            break;
                        }
                    }
                }

                if let Some(total_ms) = ready_ms {
                    // Try to fetch real metrics via CDP
                    let mut dns_ms = Some(total_ms / 10);
                    let mut tcp_ms = Some(total_ms / 5);
                    let mut dom_ms = Some(total_ms);
                    let mut load_ms = Some(total_ms + 200);

                    if let Some(real) = cdp::fetch_real_metrics(port).await {
                        println!("[Lib] Real CDP metrics fetched for {}", profile_id);
                        dns_ms = Some(real.dns_duration_ms);
                        tcp_ms = Some(real.tcp_duration_ms);
                        dom_ms = Some(real.dom_ready_ms);
                        load_ms = Some(real.page_load_ms);
                    } else {
                        println!("[Lib] Failed to fetch real metrics, using estimates for {}", profile_id);
                    }

                    let record = PerformanceRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        profile_id: profile_id.clone(),
                        launch_duration_ms: total_ms,
                        spawn_duration_ms: spawn_ms,
                        dns_duration_ms: dns_ms,
                        tcp_duration_ms: tcp_ms,
                        dom_ready_ms: dom_ms,
                        page_load_ms: load_ms,
                        created_at: chrono::Utc::now().to_rfc3339(),
                    };

                    let db_lock = db_clone.lock().unwrap();
                    let _ = db_lock.save_performance_record(&record);
                    println!("[Lib] Performance record saved for {}: {}ms", profile_id, total_ms);
                }
            });
        }
    }

    Ok(result)
}

#[tauri::command]
async fn backup_profile(
    app_handle: AppHandle,
    id: String,
    backup_dir: String,
    state: State<'_, AppState>,
) -> AppResult<BackupResult> {
    let (profile_dir, profile_name) = {
        let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let profile = db.get_profile_by_id(&id)?
            .ok_or(AppError::ProfileNotFound)?;
        (PathBuf::from(&profile.data_dir_path), profile.name.clone())
    };
    
    let backup_path = PathBuf::from(&backup_dir);
    
    // Perform heavy backup operation in a blocking thread to keep the async executor free
    let result = tauri::async_runtime::spawn_blocking(move || {
        ProfileManager::backup_profile(&profile_dir, &backup_path, &profile_name)
    }).await.map_err(|e| AppError::Other(e.to_string()))?;
    
    if result.success {
        if let Some(backup_file_path) = &result.backup_path {
            let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
            let _ = db.create_backup(&id, backup_file_path, result.size_bytes);
            prune_backups_for(&app_handle, &db, &id);
        }
    }
    
    Ok(result)
}

#[tauri::command]
async fn restore_profile(
    id: String,
    backup_path: String,
    state: State<'_, AppState>,
) -> AppResult<RestoreResult> {
    let target_dir = {
        let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let profile = db.get_profile_by_id(&id)?
            .ok_or(AppError::ProfileNotFound)?;
        PathBuf::from(&profile.data_dir_path)
    };
    
    let backup_file = PathBuf::from(&backup_path);
    
    // Perform heavy restore operation in a blocking thread
    let result = tauri::async_runtime::spawn_blocking(move || {
        ProfileManager::restore_profile(&backup_file, &target_dir)
    }).await.map_err(|e| AppError::Other(e.to_string()))?;
    
    Ok(result)
}

#[tauri::command]
fn get_backups(id: String, state: State<AppState>) -> AppResult<Vec<Backup>> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.get_backups_by_profile(&id)?)
}

#[tauri::command]
fn delete_backup(id: String, state: State<AppState>) -> AppResult<bool> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    
    // Fetch backup info to get the file path first
    if let Ok(Some(backup)) = db.get_backup_by_id(&id) {
        let path = PathBuf::from(&backup.backup_path);
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }
    
    Ok(db.delete_backup(&id)?)
}

#[tauri::command]
fn search_profiles(query: String, state: State<AppState>) -> AppResult<Vec<Profile>> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.search_profiles(&query)?)
}

#[tauri::command]
fn get_profiles_by_tag(tag: String, state: State<AppState>) -> AppResult<Vec<Profile>> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.get_profiles_by_tag(&tag)?)
}

#[tauri::command]
fn get_performance_logs(id: String, limit: Option<i32>, state: State<AppState>) -> AppResult<Vec<PerformanceRecord>> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.get_performance_logs(&id, limit.unwrap_or(10))?)
}

#[tauri::command]
fn get_profile_size(id: String, state: State<AppState>) -> AppResult<String> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    
    if let Ok(Some(profile)) = db.get_profile_by_id(&id) {
        let profile_dir = PathBuf::from(&profile.data_dir_path);
        if let Ok(size) = ProfileManager::get_directory_size(&profile_dir) {
            return Ok(ProfileManager::format_size(size));
        }
    }
    
    Ok("0 B".to_string())
}

#[tauri::command]
fn get_app_data_dir(app_handle: AppHandle) -> AppResult<String> {
    Ok(app_handle.path()
        .app_data_dir()?
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
fn open_profile_directory(profile_data_dir: String) -> AppResult<()> {
    let path = PathBuf::from(&profile_data_dir);
    if !path.exists() {
        return Err(AppError::InvalidPath("目录不存在".to_string()));
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()?;
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()?;
    }
    
    Ok(())
}

#[tauri::command]
fn unlock_profile(id: String, state: State<AppState>) -> AppResult<()> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    let profile = db.get_profile_by_id(&id)?
        .ok_or(AppError::ProfileNotFound)?;
    
    let profile_dir = PathBuf::from(&profile.data_dir_path);
    state.chrome_manager.unlock_profile(&profile_dir)
        .map_err(|e| AppError::Other(e))?;
    
    Ok(())
}

#[tauri::command]
async fn clear_profile_cache(id: String, state: State<'_, AppState>) -> AppResult<u64> {
    let (profile_dir, is_running) = {
        let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let profile = db.get_profile_by_id(&id)?.ok_or(AppError::ProfileNotFound)?;
        (PathBuf::from(&profile.data_dir_path), profile.is_running)
    };

    // 运行中目录里的缓存文件被浏览器占用，删不掉且可能损坏状态
    if is_running {
        return Err(AppError::Other("请先关闭该配置，再清理缓存".to_string()));
    }

    tauri::async_runtime::spawn_blocking(move || {
        ProfileManager::clear_profile_cache(&profile_dir)
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))
}

#[tauri::command]
fn get_paths(app_handle: AppHandle) -> AppResult<PathsInfo> {
    let app_data_dir = app_data_dir_of(&app_handle)?;
    let profiles_dir = profiles_dir_of(&app_handle)?;
    let backup_dir = backup_dir_of(&app_handle)?;
    let config = load_config(&app_handle);

    Ok(PathsInfo {
        app_data_dir: app_data_dir.to_string_lossy().to_string(),
        profiles_dir: profiles_dir.to_string_lossy().to_string(),
        backup_dir: backup_dir.to_string_lossy().to_string(),
        using_custom_data_dir: config.data_dir.is_some(),
        using_custom_backup_dir: config.backup_dir.is_some(),
        backup_retention: config.backup_retention.unwrap_or(0),
    })
}

/// 设置每个配置保留的备份数量，0 = 不限制。立即按新策略清理一次
#[tauri::command]
fn set_backup_retention(app_handle: AppHandle, keep: usize, state: State<AppState>) -> AppResult<PathsInfo> {
    {
        let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;

        // 先取全部配置 id，避免清理过程中反复锁库
        let ids: Vec<String> = db.get_all_profiles()?.into_iter().map(|p| p.id).collect();

        if keep > 0 {
            let mut stale: Vec<String> = Vec::new();

            for id in &ids {
                stale.extend(db.prune_backups(id, keep)?);
            }

            for path in stale {
                remove_backup_file(&app_handle, &path);
            }
        }
    }

    let mut config = load_config(&app_handle);
    config.backup_retention = Some(keep);
    save_config(&app_handle, &config)?;

    get_paths(app_handle)
}

/// 无边框窗口的窗口操作。Tauri v2 并未实现 data-tauri-drag-region 属性，
/// 拖动与控制按钮都必须自己调用底层接口
#[tauri::command]
fn window_action(app_handle: AppHandle, action: String) -> AppResult<()> {
    let Some(window) = app_handle.get_webview_window("main") else {
        return Ok(());
    };

    match action.as_str() {
        "minimize" => {
            let _ = window.minimize();
        }
        "maximize" => {
            // WebviewWindow 没有 toggle，按当前状态自己切换
            match window.is_maximized() {
                Ok(true) => {
                    let _ = window.unmaximize();
                }
                _ => {
                    let _ = window.maximize();
                }
            }
        }
        "close" => {
            let _ = window.close();
        }
        "drag" => {
            let _ = window.start_dragging();
        }
        _ => {}
    }

    Ok(())
}

/// 记住窗口大小与位置，下次启动恢复
#[tauri::command]
fn save_window_state(app_handle: AppHandle) -> AppResult<()> {
    let Some(window) = app_handle.get_webview_window("main") else {
        return Ok(());
    };

    let scale = window.scale_factor().unwrap_or(1.0);

    let Ok(size) = window.inner_size() else {
        return Ok(());
    };
    let Ok(position) = window.outer_position() else {
        return Ok(());
    };

    let mut config = load_config(&app_handle);
    config.window = Some(WindowState {
        width: size.width as f64 / scale,
        height: size.height as f64 / scale,
        x: position.x as f64 / scale,
        y: position.y as f64 / scale,
    });
    save_config(&app_handle, &config)?;

    Ok(())
}

#[tauri::command]
async fn set_data_dir(
    app_handle: AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> AppResult<PathsInfo> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidPath("目录不能为空".to_string()));
    }

    if running_profile_count(&state)? > 0 {
        return Err(AppError::Other(
            "有配置正在运行，请先全部关闭后再搬迁数据目录".to_string(),
        ));
    }

    let new_dir = PathBuf::from(trimmed);
    let old_dir = profiles_dir_of(&app_handle)?;

    // 已指向同一位置（例如通过 junction 指向的目标）只记录配置，不搬迁
    let same_location = fs::canonicalize(&old_dir)
        .and_then(|old| fs::canonicalize(&new_dir).map(|new| old == new))
        .unwrap_or(false);

    if !same_location {
        let old_prefix = old_dir.to_string_lossy().to_string();
        let new_prefix = new_dir.to_string_lossy().to_string();

        let handle = app_handle.clone();
        let move_from = old_dir.clone();
        let move_to = new_dir.clone();

        let (moved, _skipped) = tauri::async_runtime::spawn_blocking(move || {
            fs::create_dir_all(&move_to)?;

            let (moved, skipped) = move_dir_contents_with_progress(
                &move_from,
                &move_to,
                |current, total, label| {
                    let _ = handle.emit(
                        "migration-progress",
                        MigrationProgress {
                            current,
                            total,
                            label,
                        },
                    );
                },
            );

            Ok::<(usize, usize), std::io::Error>((moved, skipped))
        })
        .await
        .map_err(|e| AppError::Other(e.to_string()))?
        .map_err(|e| AppError::Other(e.to_string()))?;

        // 文件搬走了，数据库里的绝对路径必须跟着改，否则配置会找不到自己的目录
        if moved > 0 {
            let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
            db.rewrite_data_dir_prefix(&old_prefix, &new_prefix)?;
        }
    }

    let mut config = load_config(&app_handle);
    config.data_dir = Some(new_dir.to_string_lossy().to_string());
    save_config(&app_handle, &config)?;

    get_paths(app_handle)
}

#[tauri::command]
async fn set_backup_dir(app_handle: AppHandle, path: String) -> AppResult<PathsInfo> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidPath("目录不能为空".to_string()));
    }

    let new_dir = PathBuf::from(trimmed);
    let old_dir = backup_dir_of(&app_handle)?;

    let same_location = fs::canonicalize(&old_dir)
        .and_then(|old| fs::canonicalize(&new_dir).map(|new| old == new))
        .unwrap_or(false);

    if !same_location && old_dir.exists() {
        let handle = app_handle.clone();
        let move_from = old_dir.clone();
        let move_to = new_dir.clone();

        tauri::async_runtime::spawn_blocking(move || {
            fs::create_dir_all(&move_to)?;

            move_dir_contents_with_progress(&move_from, &move_to, |current, total, label| {
                let _ = handle.emit(
                    "migration-progress",
                    MigrationProgress {
                        current,
                        total,
                        label,
                    },
                );
            });

            Ok::<(), std::io::Error>(())
        })
        .await
        .map_err(|e| AppError::Other(e.to_string()))?
        .map_err(|e| AppError::Other(e.to_string()))?;
    } else {
        fs::create_dir_all(&new_dir)?;
    }

    let mut config = load_config(&app_handle);
    config.backup_dir = Some(new_dir.to_string_lossy().to_string());
    save_config(&app_handle, &config)?;

    get_paths(app_handle)
}

#[derive(serde::Serialize)]
struct ClearCacheSummary {
    freed_bytes: u64,
    cleared: usize,
    skipped: usize,
}

#[tauri::command]
async fn clear_profiles_cache(
    ids: Vec<String>,
    state: State<'_, AppState>,
) -> AppResult<ClearCacheSummary> {
    let targets = {
        let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let mut targets = Vec::new();

        for id in &ids {
            if let Some(profile) = db.get_profile_by_id(id)? {
                targets.push((profile.data_dir_path, profile.is_running));
            }
        }

        targets
    };

    let (freed_bytes, cleared, skipped) = tauri::async_runtime::spawn_blocking(move || {
        let mut freed = 0u64;
        let mut cleared = 0usize;
        let mut skipped = 0usize;

        for (data_dir, is_running) in targets {
            let dir = PathBuf::from(data_dir);

            // 运行中的配置跳过，交给用户先关闭
            if is_running || !dir.exists() {
                skipped += 1;
                continue;
            }

            freed += ProfileManager::clear_profile_cache(&dir);
            cleared += 1;
        }

        (freed, cleared, skipped)
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?;

    Ok(ClearCacheSummary {
        freed_bytes,
        cleared,
        skipped,
    })
}

#[tauri::command]
async fn get_profile_sizes(state: State<'_, AppState>) -> AppResult<Vec<(String, u64)>> {
    let profiles = {
        let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
        db.get_all_profiles()?
    };

    let dirs: Vec<(String, PathBuf)> = profiles
        .into_iter()
        .map(|p| (p.id, PathBuf::from(&p.data_dir_path)))
        .collect();

    tauri::async_runtime::spawn_blocking(move || {
        dirs.into_iter()
            .map(|(id, dir)| (id, ProfileManager::get_directory_size(&dir).unwrap_or(0)))
            .collect::<Vec<(String, u64)>>()
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))
}

#[tauri::command]
fn reorder_profiles(ids: Vec<String>, state: State<AppState>) -> AppResult<()> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    db.reorder_profiles(&ids)?;
    Ok(())
}

#[tauri::command]
fn get_workspaces(state: State<AppState>) -> AppResult<Vec<Workspace>> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.get_all_workspaces()?)
}

#[tauri::command]
fn create_workspace(
    name: String,
    icon: Option<String>,
    color: Option<String>,
    description: Option<String>,
    state: State<AppState>,
) -> AppResult<Workspace> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.create_workspace(&name, icon.as_deref(), color.as_deref(), description.as_deref())?)
}

#[tauri::command]
fn update_workspace(
    id: String,
    name: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    description: Option<String>,
    state: State<AppState>,
) -> AppResult<bool> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.update_workspace(&id, name.as_deref(), icon.as_deref(), color.as_deref(), description.as_deref())?)
}

#[tauri::command]
fn delete_workspace(id: String, state: State<AppState>) -> AppResult<bool> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.delete_workspace(&id)?)
}

#[tauri::command]
fn get_profiles_by_workspace(workspace_id: String, state: State<AppState>) -> AppResult<Vec<Profile>> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.get_profiles_by_workspace(&workspace_id)?)
}

#[tauri::command]
fn move_profile_to_workspace(
    id: String,
    workspace_id: Option<String>,
    state: State<AppState>,
) -> AppResult<bool> {
    let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(db.set_profile_workspace(&id, workspace_id.as_deref())?)
}

#[tauri::command]
async fn backup_workspace(
    app_handle: AppHandle,
    workspace_id: String,
    backup_dir: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<BackupResult>> {
    let profiles = {
        let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;
        db.get_profiles_by_workspace(&workspace_id)?
    };

    run_backup(app_handle, state.db.clone(), profiles, backup_dir).await
}

/// 批量备份指定 id 的配置：一次调用完成，避免前端逐个往返
#[tauri::command]
async fn backup_profiles(
    app_handle: AppHandle,
    ids: Vec<String>,
    backup_dir: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<BackupResult>> {
    let profiles = {
        let db = state.db.lock().map_err(|e| AppError::Other(e.to_string()))?;

        let mut list = Vec::new();
        for id in ids {
            if let Some(profile) = db.get_profile_by_id(&id)? {
                list.push(profile);
            }
        }

        list
    };

    run_backup(app_handle, state.db.clone(), profiles, backup_dir).await
}

/// 逐个打包并在打包前上报进度，让界面能显示「正在备份 3/8」
async fn run_backup(
    app_handle: AppHandle,
    db: Arc<Mutex<Database>>,
    profiles: Vec<Profile>,
    backup_dir: String,
) -> AppResult<Vec<BackupResult>> {
    tauri::async_runtime::spawn_blocking(move || {
        let backup_path = PathBuf::from(&backup_dir);
        let mut results = Vec::new();
        let total = profiles.len();

        for (index, profile) in profiles.into_iter().enumerate() {
            let _ = app_handle.emit(
                "backup-progress",
                BackupProgress {
                    current: index + 1,
                    total,
                    name: profile.name.clone(),
                },
            );

            let profile_dir = PathBuf::from(&profile.data_dir_path);
            let result = ProfileManager::backup_profile(&profile_dir, &backup_path, &profile.name);

            if result.success {
                if let Some(path) = &result.backup_path {
                    if let Ok(guard) = db.lock() {
                        let _ = guard.create_backup(&profile.id, path, result.size_bytes);
                        // 写入记录后再按保留策略清理，保证删除的始终是最旧的
                        prune_backups_for(&app_handle, &guard, &profile.id);
                    }
                }
            }

            results.push(result);
        }

        results
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            
            let db = Arc::new(Mutex::new(Database::new(&app_data_dir)?));
            let chrome_manager = ChromeManager::new();
            
            let monitor_db = db.clone();
            let app_handle = app.handle().clone();
            let monitor = BrowserMonitor::new(monitor_db, app_handle);
            monitor.start_monitoring();
            
            app.manage(AppState {
                db,
                chrome_manager,
            });

            // 恢复上次关闭时的窗口大小与位置
            if let Some(window) = app.get_webview_window("main") {
                if let Some(saved) = load_config(app.handle()).window {
                    let w = saved.width.max(640.0);
                    let h = saved.height.max(480.0);

                    let _ = window.set_size(tauri::LogicalSize::new(w, h));
                    let _ = window.set_position(tauri::LogicalPosition::new(saved.x, saved.y));
                }

                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        let _ = save_window_state(handle.clone());
                    }
                });
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_profiles,
            get_profile,
            create_profile,
            update_profile,
            delete_profile,
            launch_chrome,
            backup_profile,
            restore_profile,
            get_backups,
            delete_backup,
            search_profiles,
            get_profiles_by_tag,
            get_performance_logs,
            get_profile_size,
            get_app_data_dir,
            open_profile_directory,
            unlock_profile,
            get_workspaces,
            create_workspace,
            update_workspace,
            delete_workspace,
            get_profiles_by_workspace,
            move_profile_to_workspace,
            backup_workspace,
            clear_profile_cache,
            clear_profiles_cache,
            reorder_profiles,
            set_profile_template,
            get_template_ids,
            get_profile_sizes,
            get_paths,
            set_data_dir,
            set_backup_dir,
            set_backup_retention,
            save_window_state,
            window_action,
            move_profiles_to_workspace,
            backup_profiles,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
