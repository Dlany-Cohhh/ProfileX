use rusqlite::{Connection, Result as SqliteResult, Row};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use chrono::Utc;

/// 浏览器启动参数，以 JSON 形式存在 profiles.launch_args 列
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct LaunchArgs {
    pub proxy: Option<String>,
    pub user_agent: Option<String>,
    pub window_size: Option<String>,
    pub window_position: Option<String>,
    pub language: Option<String>,
    pub extra_args: Option<String>,
}

impl LaunchArgs {
    /// 转成 Chrome 命令行参数；空值不产生参数，额外参数按空白拆分
    pub fn to_cli_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        let mut push = |flag: &str, value: &Option<String>| {
            if let Some(v) = value {
                let v = v.trim();
                if !v.is_empty() {
                    args.push(format!("{}={}", flag, v));
                }
            }
        };

        push("--proxy-server", &self.proxy);
        push("--user-agent", &self.user_agent);
        push("--window-size", &self.window_size);
        push("--window-position", &self.window_position);
        push("--lang", &self.language);

        if let Some(extra) = &self.extra_args {
            for part in extra.split_whitespace() {
                args.push(part.to_string());
            }
        }

        args
    }
}

/// 统一的列读取入口：按列名取值。
/// 之前用 row.get(15) 这种下标，新增字段时一旦 INSERT 与 SELECT 的列序不一致就会静默读串。
fn profile_from_row(row: &Row) -> SqliteResult<Profile> {
    let launch_args_json: Option<String> = row.get("launch_args")?;

    Ok(Profile {
        id: row.get("id")?,
        name: row.get("name")?,
        data_dir_path: row.get("data_dir_path")?,
        chrome_path: row.get("chrome_path")?,
        homepage: row.get("homepage")?,
        icon_path: row.get("icon_path")?,
        icon_base64: row.get("icon_base64")?,
        tags: row.get("tags")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        last_opened_at: row.get("last_opened_at")?,
        is_running: row.get::<_, i32>("is_running")? != 0,
        pid: row.get("pid")?,
        debug_port: row.get::<_, Option<i32>>("debug_port")?.map(|v| v as u16),
        forward_port: row.get::<_, Option<i32>>("forward_port")?.map(|v| v as u16),
        disable_extensions: row.get::<_, i32>("disable_extensions")? != 0,
        debugger_mode: row.get::<_, i32>("debugger_mode")? != 0,
        debug_port_config: row.get::<_, Option<i32>>("debug_port_config")?.map(|v| v as u16),
        workspace_id: row.get("workspace_id")?,
        launch_args: launch_args_json
            .and_then(|json| serde_json::from_str::<LaunchArgs>(&json).ok())
            .unwrap_or_default(),
        is_template: row.get::<_, i32>("is_template")? != 0,
        sort_order: row.get("sort_order")?,
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub data_dir_path: String,
    pub chrome_path: Option<String>,
    pub homepage: Option<String>,
    pub icon_path: Option<String>,
    pub icon_base64: Option<String>,
    pub tags: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_opened_at: Option<String>,
    pub is_running: bool,
    pub pid: Option<i32>,
    pub debug_port: Option<u16>,
    pub forward_port: Option<u16>,
    pub disable_extensions: bool,
    pub debugger_mode: bool,
    pub debug_port_config: Option<u16>,
    pub workspace_id: Option<String>,
    pub launch_args: LaunchArgs,
    pub is_template: bool,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Backup {
    pub id: String,
    pub profile_id: String,
    pub backup_path: String,
    pub created_at: String,
    pub size_bytes: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerformanceRecord {
    pub id: String,
    pub profile_id: String,
    pub launch_duration_ms: u64,
    pub spawn_duration_ms: u64,
    pub dns_duration_ms: Option<u64>,
    pub tcp_duration_ms: Option<u64>,
    pub dom_ready_ms: Option<u64>,
    pub page_load_ms: Option<u64>,
    pub created_at: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(app_data_dir: &PathBuf) -> SqliteResult<Self> {
        let db_path = app_data_dir.join("profiles.db");
        let conn = Connection::open(&db_path)?;
        
        let db = Database { conn };
        db.init_tables()?;
        
        Ok(db)
    }

    fn init_tables(&self) -> SqliteResult<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data_dir_path TEXT NOT NULL UNIQUE,
                chrome_path TEXT,
                homepage TEXT,
                icon_path TEXT,
                icon_base64 TEXT,
                tags TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_opened_at TEXT,
                is_running INTEGER DEFAULT 0,
                pid INTEGER,
                debug_port INTEGER,
                forward_port INTEGER,
                disable_extensions INTEGER NOT NULL DEFAULT 1,
                debugger_mode INTEGER NOT NULL DEFAULT 1,
                debug_port_config INTEGER
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS performance_logs (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                launch_duration_ms INTEGER NOT NULL,
                spawn_duration_ms INTEGER NOT NULL,
                dns_duration_ms INTEGER,
                tcp_duration_ms INTEGER,
                dom_ready_ms INTEGER,
                page_load_ms INTEGER,
                created_at TEXT NOT NULL,
                FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS backups (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                backup_path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_profiles_name ON profiles(name)",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                icon TEXT,
                color TEXT,
                description TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_profiles_tags ON profiles(tags)",
            [],
        )?;

        // Migrations
        let _ = self.conn.execute("ALTER TABLE profiles ADD COLUMN chrome_path TEXT", []);
        let _ = self.conn.execute("ALTER TABLE profiles ADD COLUMN homepage TEXT", []);
        let _ = self.conn.execute("ALTER TABLE profiles ADD COLUMN debug_port INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE profiles ADD COLUMN forward_port INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE profiles RENAME COLUMN custom_cdp_port TO forward_port", []);
        let _ = self.conn.execute("ALTER TABLE profiles ADD COLUMN disable_extensions INTEGER NOT NULL DEFAULT 1", []);
        let _ = self.conn.execute("ALTER TABLE profiles ADD COLUMN debugger_mode INTEGER NOT NULL DEFAULT 1", []);
        let _ = self.conn.execute("ALTER TABLE profiles ADD COLUMN debug_port_config INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE profiles ADD COLUMN workspace_id TEXT", []);
        let _ = self.conn.execute("ALTER TABLE profiles ADD COLUMN sort_order INTEGER", []);
        let _ = self
            .conn
            .execute("ALTER TABLE profiles ADD COLUMN is_template INTEGER NOT NULL DEFAULT 0", []);
        let _ = self
            .conn
            .execute("ALTER TABLE profiles ADD COLUMN launch_args TEXT", []);

        self.backfill_sort_order()?;

        // 索引必须在列迁移之后创建，否则旧数据库上会因列不存在而失败
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_profiles_workspace ON profiles(workspace_id)",
            [],
        )?;

        Ok(())
    }

    /// 给尚无排序值的配置按当前展示顺序（updated_at DESC）补号，保证升级后界面顺序不变
    fn backfill_sort_order(&self) -> SqliteResult<()> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM profiles WHERE sort_order IS NULL ORDER BY updated_at DESC")?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<SqliteResult<Vec<String>>>()?;
        drop(stmt);

        for (index, id) in ids.iter().enumerate() {
            self.conn.execute(
                "UPDATE profiles SET sort_order = ?1 WHERE id = ?2",
                (index as i64, id),
            )?;
        }

        Ok(())
    }

    /// 批量移动配置到同一工作区，一次调用完成，避免前端逐个往返
    pub fn move_profiles_to_workspace(&self, ids: &[String], workspace_id: Option<&str>) -> SqliteResult<usize> {
        let now = Utc::now().to_rfc3339();
        let mut moved = 0usize;

        for id in ids {
            let changed = self.conn.execute(
                "UPDATE profiles SET workspace_id = ?1, updated_at = ?2 WHERE id = ?3",
                (workspace_id, &now, id),
            )?;

            moved += changed;
        }

        Ok(moved)
    }

    /// 备份保留策略：只保留每个配置最近的 keep 个备份，返回被删除的文件路径
    /// keep = 0 表示不限制，不动任何文件
    pub fn prune_backups(&self, profile_id: &str, keep: usize) -> SqliteResult<Vec<String>> {
        if keep == 0 {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "SELECT id, backup_path FROM backups WHERE profile_id = ?1
             ORDER BY created_at DESC",
        )?;

        let rows: Vec<(String, String)> = stmt
            .query_map([profile_id], |row| Ok((row.get("id")?, row.get("backup_path")?)))?
            .collect::<SqliteResult<Vec<(String, String)>>>()?;
        drop(stmt);

        let mut removed = Vec::new();

        for (index, (backup_id, path)) in rows.into_iter().enumerate() {
            if index < keep {
                continue;
            }

            self.conn.execute("DELETE FROM backups WHERE id = ?1", [&backup_id])?;
            removed.push(path);
        }

        Ok(removed)
    }

    /// 标记 / 取消模板配置
    pub fn set_profile_template(&self, profile_id: &str, is_template: bool) -> SqliteResult<bool> {
        let affected = self.conn.execute(
            "UPDATE profiles SET is_template = ?1, updated_at = ?2 WHERE id = ?3",
            (
                if is_template { 1 } else { 0 },
                Utc::now().to_rfc3339(),
                profile_id,
            ),
        )?;

        Ok(affected > 0)
    }

    /// 所有被标记为模板的配置 id
    pub fn get_template_ids(&self) -> SqliteResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM profiles WHERE is_template = 1 ORDER BY sort_order ASC, updated_at DESC",
        )?;

        let ids = stmt
            .query_map([], |row| row.get(0))?
            .collect::<SqliteResult<Vec<String>>>()?;

        Ok(ids)
    }

    /// 整体重写排序：数组下标即新的 sort_order
    pub fn reorder_profiles(&self, ids: &[String]) -> SqliteResult<()> {
        for (index, id) in ids.iter().enumerate() {
            self.conn.execute(
                "UPDATE profiles SET sort_order = ?1 WHERE id = ?2",
                (index as i64, id),
            )?;
        }

        Ok(())
    }

    pub fn create_profile(&self, name: &str, data_dir_path: &str, chrome_path: Option<&str>, homepage: Option<&str>, icon_base64: Option<&str>, tags: Option<&str>, forward_port: Option<u16>, disable_extensions: bool, debugger_mode: bool, debug_port_config: Option<u16>, workspace_id: Option<&str>, launch_args: Option<&LaunchArgs>) -> SqliteResult<Profile> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // 新配置排到最前（最小 sort_order - 1），与原先「最近更新的在前」观感一致
        let min_order: Option<i64> = self
            .conn
            .query_row("SELECT MIN(sort_order) FROM profiles", [], |row| row.get(0))
            .unwrap_or(None);

        let sort_order = min_order.unwrap_or(0) - 1;

        // 启动参数整存 JSON，避免每加一个开关都要加列、改 INSERT 与所有 SELECT
        let launch_args_json = launch_args
            .and_then(|args| serde_json::to_string(args).ok());

        self.conn.execute(
            "INSERT INTO profiles (id, name, data_dir_path, chrome_path, homepage, icon_base64, tags, forward_port, disable_extensions, debugger_mode, debug_port_config, workspace_id, sort_order, launch_args, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            (&id, name, data_dir_path, chrome_path, homepage, icon_base64, tags, forward_port.map(|v| v as i32), if disable_extensions { 1 } else { 0 }, if debugger_mode { 1 } else { 0 }, debug_port_config.map(|v| v as i32), workspace_id, sort_order, &launch_args_json, &now, &now),
        )?;

        Ok(Profile {
            id,
            name: name.to_string(),
            data_dir_path: data_dir_path.to_string(),
            chrome_path: chrome_path.map(|s| s.to_string()),
            homepage: homepage.map(|s| s.to_string()),
            icon_path: None,
            icon_base64: icon_base64.map(|s| s.to_string()),
            tags: tags.map(|s| s.to_string()),
            created_at: now.clone(),
            updated_at: now,
            last_opened_at: None,
            is_running: false,
            pid: None,
            debug_port: None,
            forward_port,
            disable_extensions,
            debugger_mode,
            debug_port_config,
            workspace_id: workspace_id.map(|s| s.to_string()),
            launch_args: launch_args.cloned().unwrap_or_default(),
            is_template: false,
            sort_order: Some(sort_order),
        })
    }

    /// 导入备份时整条写入：连 id、排序、模板标记都按备份里的原样恢复
    #[allow(clippy::too_many_arguments)]
    pub fn import_profile(
        &self,
        id: &str,
        name: &str,
        data_dir_path: &str,
        chrome_path: Option<&str>,
        homepage: Option<&str>,
        icon_base64: Option<&str>,
        tags: Option<&str>,
        forward_port: Option<u16>,
        disable_extensions: bool,
        debugger_mode: bool,
        debug_port_config: Option<u16>,
        workspace_id: Option<&str>,
        launch_args: Option<&LaunchArgs>,
        is_template: bool,
        sort_order: i64,
    ) -> SqliteResult<Profile> {
        let now = Utc::now().to_rfc3339();
        let launch_args_json = launch_args.and_then(|args| serde_json::to_string(args).ok());

        self.conn.execute(
            "INSERT INTO profiles (id, name, data_dir_path, chrome_path, homepage, icon_base64, tags, forward_port, disable_extensions, debugger_mode, debug_port_config, workspace_id, sort_order, launch_args, is_template, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            // 超过 16 个参数，元组不再实现 Params，改用 params!
            rusqlite::params![
                id, name, data_dir_path, chrome_path, homepage, icon_base64, tags,
                forward_port.map(|v| v as i32),
                if disable_extensions { 1 } else { 0 },
                if debugger_mode { 1 } else { 0 },
                debug_port_config.map(|v| v as i32),
                workspace_id, sort_order, &launch_args_json,
                if is_template { 1 } else { 0 },
                &now, &now
            ],
        )?;

        Ok(Profile {
            id: id.to_string(),
            name: name.to_string(),
            data_dir_path: data_dir_path.to_string(),
            chrome_path: chrome_path.map(|s| s.to_string()),
            homepage: homepage.map(|s| s.to_string()),
            icon_path: None,
            icon_base64: icon_base64.map(|s| s.to_string()),
            tags: tags.map(|s| s.to_string()),
            created_at: now.clone(),
            updated_at: now,
            last_opened_at: None,
            is_running: false,
            pid: None,
            debug_port: None,
            forward_port,
            disable_extensions,
            debugger_mode,
            debug_port_config,
            workspace_id: workspace_id.map(|s| s.to_string()),
            launch_args: launch_args.cloned().unwrap_or_default(),
            is_template,
            sort_order: Some(sort_order),
        })
    }

    /// 当前最小的排序值，新配置排到最前时要用到
    pub fn min_sort_order(&self) -> Option<i64> {
        self.conn
            .query_row("SELECT MIN(sort_order) FROM profiles", [], |row| row.get(0))
            .unwrap_or(None)
    }

    /// 工作区按 id 还原：已存在就直接复用，否则用备份里的 id 与信息新建
    pub fn ensure_workspace(
        &self,
        id: &str,
        name: &str,
        icon: Option<&str>,
        color: Option<&str>,
        description: Option<&str>,
    ) -> SqliteResult<String> {
        let exists: Option<String> = self
            .conn
            .query_row("SELECT id FROM workspaces WHERE id = ?1", [id], |row| row.get(0))
            .ok();

        if let Some(found) = exists {
            return Ok(found);
        }

        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO workspaces (id, name, icon, color, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (id, name, icon, color, description, &now, &now),
        )?;

        Ok(id.to_string())
    }

    pub fn get_all_profiles(&self) -> SqliteResult<Vec<Profile>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, data_dir_path, chrome_path, homepage, icon_path, icon_base64, tags,
                    created_at, updated_at, last_opened_at, is_running, pid, debug_port, forward_port, disable_extensions, debugger_mode, debug_port_config, workspace_id, launch_args, is_template, sort_order
             FROM profiles ORDER BY sort_order ASC, updated_at DESC"
        )?;

        let profiles = stmt.query_map([], profile_from_row)?;

        profiles.collect()
    }

    pub fn get_profile_by_id(&self, id: &str) -> SqliteResult<Option<Profile>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, data_dir_path, chrome_path, homepage, icon_path, icon_base64, tags,
                    created_at, updated_at, last_opened_at, is_running, pid, debug_port, forward_port, disable_extensions, debugger_mode, debug_port_config, workspace_id, launch_args, is_template, sort_order
             FROM profiles WHERE id = ?1"
        )?;

        let mut rows = stmt.query_map([id], profile_from_row)?;

        rows.next().transpose()
    }

    pub fn update_profile(&self, id: &str, name: Option<&str>, chrome_path: Option<&str>, homepage: Option<&str>, icon_base64: Option<&str>, tags: Option<&str>, forward_port: Option<u16>, disable_extensions: Option<bool>, debugger_mode: Option<bool>, debug_port_config: Option<u16>, launch_args: Option<&LaunchArgs>) -> SqliteResult<bool> {
        let now = Utc::now().to_rfc3339();

        if let Some(name) = name {
            self.conn.execute(
                "UPDATE profiles SET name = ?1, updated_at = ?2 WHERE id = ?3",
                (name, &now, id),
            )?;
        }

        if let Some(path) = chrome_path {
            self.conn.execute(
                "UPDATE profiles SET chrome_path = ?1, updated_at = ?2 WHERE id = ?3",
                (path, &now, id),
            )?;
        }

        if let Some(url) = homepage {
            self.conn.execute(
                "UPDATE profiles SET homepage = ?1, updated_at = ?2 WHERE id = ?3",
                (url, &now, id),
            )?;
        }

        if let Some(icon) = icon_base64 {
            self.conn.execute(
                "UPDATE profiles SET icon_base64 = ?1, updated_at = ?2 WHERE id = ?3",
                (icon, &now, id),
            )?;
        }

        if let Some(tags) = tags {
            self.conn.execute(
                "UPDATE profiles SET tags = ?1, updated_at = ?2 WHERE id = ?3",
                (tags, &now, id),
            )?;
        }

        if let Some(disable) = disable_extensions {
            self.conn.execute(
                "UPDATE profiles SET disable_extensions = ?1, updated_at = ?2 WHERE id = ?3",
                (if disable { 1 } else { 0 }, &now, id),
            )?;
        }

        if let Some(debugger) = debugger_mode {
            self.conn.execute(
                "UPDATE profiles SET debugger_mode = ?1, updated_at = ?2 WHERE id = ?3",
                (if debugger { 1 } else { 0 }, &now, id),
            )?;
        }

        self.conn.execute(
            "UPDATE profiles SET forward_port = ?1, debug_port_config = ?2, updated_at = ?3 WHERE id = ?4",
            (forward_port.map(|v| v as i32), debug_port_config.map(|v| v as i32), &now, id),
        )?;

        if let Some(args) = launch_args {
            let json = serde_json::to_string(args).ok();
            self.conn.execute(
                "UPDATE profiles SET launch_args = ?1, updated_at = ?2 WHERE id = ?3",
                (&json, &now, id),
            )?;
        }

        Ok(self.conn.changes() > 0)
    }

    pub fn delete_profile(&self, id: &str) -> SqliteResult<bool> {
        self.conn.execute("DELETE FROM profiles WHERE id = ?1", [id])?;
        Ok(self.conn.changes() > 0)
    }

    pub fn update_profile_status(&self, id: &str, is_running: bool, pid: Option<i32>, debug_port: Option<u16>) -> SqliteResult<bool> {
        let now = Utc::now().to_rfc3339();
        let running = if is_running { 1 } else { 0 };
        
        self.conn.execute(
            "UPDATE profiles SET is_running = ?1, pid = ?2, debug_port = ?3, last_opened_at = ?4, updated_at = ?5 WHERE id = ?6",
            (running, pid, debug_port.map(|v| v as i32), &now, &now, id),
        )?;

        Ok(self.conn.changes() > 0)
    }

    pub fn create_backup(&self, profile_id: &str, backup_path: &str, size_bytes: i64) -> SqliteResult<Backup> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        
        self.conn.execute(
            "INSERT INTO backups (id, profile_id, backup_path, created_at, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (&id, profile_id, backup_path, &now, size_bytes),
        )?;

        Ok(Backup {
            id,
            profile_id: profile_id.to_string(),
            backup_path: backup_path.to_string(),
            created_at: now,
            size_bytes,
        })
    }

    pub fn get_backups_by_profile(&self, profile_id: &str) -> SqliteResult<Vec<Backup>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, profile_id, backup_path, created_at, size_bytes 
             FROM backups WHERE profile_id = ?1 ORDER BY created_at DESC"
        )?;

        let backups = stmt.query_map([profile_id], |row| {
            Ok(Backup {
                id: row.get("id")?,
                profile_id: row.get("profile_id")?,
                backup_path: row.get("backup_path")?,
                created_at: row.get("created_at")?,
                size_bytes: row.get("size_bytes")?,
            })
        })?;

        backups.collect()
    }

    pub fn get_backup_by_id(&self, id: &str) -> SqliteResult<Option<Backup>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, profile_id, backup_path, created_at, size_bytes 
             FROM backups WHERE id = ?1"
        )?;

        let mut rows = stmt.query_map([id], |row| {
            Ok(Backup {
                id: row.get("id")?,
                profile_id: row.get("profile_id")?,
                backup_path: row.get("backup_path")?,
                created_at: row.get("created_at")?,
                size_bytes: row.get("size_bytes")?,
            })
        })?;

        rows.next().transpose()
    }

    pub fn delete_backup(&self, id: &str) -> SqliteResult<bool> {
        self.conn.execute("DELETE FROM backups WHERE id = ?1", [id])?;
        Ok(self.conn.changes() > 0)
    }

    pub fn search_profiles(&self, query: &str) -> SqliteResult<Vec<Profile>> {
        let search_pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, name, data_dir_path, chrome_path, homepage, icon_path, icon_base64, tags,
                    created_at, updated_at, last_opened_at, is_running, pid, debug_port, forward_port, disable_extensions, debugger_mode, debug_port_config, workspace_id, launch_args, is_template, sort_order
             FROM profiles
             WHERE name LIKE ?1 OR tags LIKE ?1
             ORDER BY updated_at DESC"
        )?;

        let profiles = stmt.query_map([&search_pattern], profile_from_row)?;

        profiles.collect()
    }

    pub fn get_profiles_by_tag(&self, tag: &str) -> SqliteResult<Vec<Profile>> {
        let search_pattern = format!("%{}%", tag);
        let mut stmt = self.conn.prepare(
            "SELECT id, name, data_dir_path, chrome_path, homepage, icon_path, icon_base64, tags,
                    created_at, updated_at, last_opened_at, is_running, pid, debug_port, forward_port, disable_extensions, debugger_mode, debug_port_config, workspace_id, launch_args, is_template, sort_order
             FROM profiles
             WHERE tags LIKE ?1
             ORDER BY updated_at DESC"
        )?;

        let profiles = stmt.query_map([&search_pattern], profile_from_row)?;

        profiles.collect()
    }

    pub fn get_all_workspaces(&self) -> SqliteResult<Vec<Workspace>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, icon, color, description, created_at, updated_at
             FROM workspaces ORDER BY created_at ASC"
        )?;

        let workspaces = stmt.query_map([], |row| {
            Ok(Workspace {
                id: row.get("id")?,
                name: row.get("name")?,
                icon: row.get("icon")?,
                color: row.get("color")?,
                description: row.get("description")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;

        workspaces.collect()
    }

    pub fn create_workspace(&self, name: &str, icon: Option<&str>, color: Option<&str>, description: Option<&str>) -> SqliteResult<Workspace> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO workspaces (id, name, icon, color, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (&id, name, icon, color, description, &now, &now),
        )?;

        Ok(Workspace {
            id,
            name: name.to_string(),
            icon: icon.map(|s| s.to_string()),
            color: color.map(|s| s.to_string()),
            description: description.map(|s| s.to_string()),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn update_workspace(&self, id: &str, name: Option<&str>, icon: Option<&str>, color: Option<&str>, description: Option<&str>) -> SqliteResult<bool> {
        let now = Utc::now().to_rfc3339();

        if let Some(name) = name {
            self.conn.execute(
                "UPDATE workspaces SET name = ?1, updated_at = ?2 WHERE id = ?3",
                (name, &now, id),
            )?;
        }

        if let Some(icon) = icon {
            self.conn.execute(
                "UPDATE workspaces SET icon = ?1, updated_at = ?2 WHERE id = ?3",
                (icon, &now, id),
            )?;
        }

        if let Some(color) = color {
            self.conn.execute(
                "UPDATE workspaces SET color = ?1, updated_at = ?2 WHERE id = ?3",
                (color, &now, id),
            )?;
        }

        if let Some(description) = description {
            self.conn.execute(
                "UPDATE workspaces SET description = ?1, updated_at = ?2 WHERE id = ?3",
                (description, &now, id),
            )?;
        }

        Ok(self.conn.changes() > 0)
    }

    pub fn delete_workspace(&self, id: &str) -> SqliteResult<bool> {
        // 删除工作区时，其下的 profile 回到「未分组」
        self.conn.execute(
            "UPDATE profiles SET workspace_id = NULL WHERE workspace_id = ?1",
            [id],
        )?;

        self.conn.execute("DELETE FROM workspaces WHERE id = ?1", [id])?;
        Ok(self.conn.changes() > 0)
    }

    pub fn get_profiles_by_workspace(&self, workspace_id: &str) -> SqliteResult<Vec<Profile>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, data_dir_path, chrome_path, homepage, icon_path, icon_base64, tags,
                    created_at, updated_at, last_opened_at, is_running, pid, debug_port, forward_port, disable_extensions, debugger_mode, debug_port_config, workspace_id, launch_args, is_template, sort_order
             FROM profiles WHERE workspace_id = ?1 ORDER BY sort_order ASC, updated_at DESC"
        )?;

        let profiles = stmt.query_map([workspace_id], profile_from_row)?;

        profiles.collect()
    }

    /// 数据目录搬迁后，把 profiles.data_dir_path 里的旧前缀替换为新前缀
    pub fn rewrite_data_dir_prefix(&self, old_prefix: &str, new_prefix: &str) -> SqliteResult<usize> {
        let mut stmt = self.conn.prepare("SELECT id, data_dir_path FROM profiles")?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<SqliteResult<Vec<(String, String)>>>()?;
        drop(stmt);

        let mut updated = 0usize;

        for (id, path) in rows {
            if let Some(rest) = path.strip_prefix(old_prefix) {
                let new_path = format!("{}{}", new_prefix, rest);
                self.conn
                    .execute("UPDATE profiles SET data_dir_path = ?1 WHERE id = ?2", (&new_path, &id))?;
                updated += 1;
            }
        }

        Ok(updated)
    }

    pub fn set_profile_workspace(&self, profile_id: &str, workspace_id: Option<&str>) -> SqliteResult<bool> {
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "UPDATE profiles SET workspace_id = ?1, updated_at = ?2 WHERE id = ?3",
            (workspace_id, &now, profile_id),
        )?;

        Ok(self.conn.changes() > 0)
    }

    pub fn save_performance_record(&self, record: &PerformanceRecord) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT INTO performance_logs (id, profile_id, launch_duration_ms, spawn_duration_ms, dns_duration_ms, tcp_duration_ms, dom_ready_ms, page_load_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (
                &record.id,
                &record.profile_id,
                record.launch_duration_ms as i64,
                record.spawn_duration_ms as i64,
                record.dns_duration_ms.map(|v| v as i64),
                record.tcp_duration_ms.map(|v| v as i64),
                record.dom_ready_ms.map(|v| v as i64),
                record.page_load_ms.map(|v| v as i64),
                &record.created_at,
            ),
        )?;
        Ok(())
    }

    pub fn get_performance_logs(&self, profile_id: &str, limit: i32) -> SqliteResult<Vec<PerformanceRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, profile_id, launch_duration_ms, spawn_duration_ms, dns_duration_ms, tcp_duration_ms, dom_ready_ms, page_load_ms, created_at
             FROM performance_logs WHERE profile_id = ?1 ORDER BY created_at DESC LIMIT ?2"
        )?;

        let logs = stmt.query_map([profile_id, &limit.to_string()], |row| {
            Ok(PerformanceRecord {
                id: row.get("id")?,
                profile_id: row.get("profile_id")?,
                launch_duration_ms: row.get::<_, i64>("launch_duration_ms")? as u64,
                spawn_duration_ms: row.get::<_, i64>("spawn_duration_ms")? as u64,
                dns_duration_ms: row.get::<_, Option<i64>>("dns_duration_ms")?.map(|v| v as u64),
                tcp_duration_ms: row.get::<_, Option<i64>>("tcp_duration_ms")?.map(|v| v as u64),
                dom_ready_ms: row.get::<_, Option<i64>>("dom_ready_ms")?.map(|v| v as u64),
                page_load_ms: row.get::<_, Option<i64>>("page_load_ms")?.map(|v| v as u64),
                created_at: row.get("created_at")?,
            })
        })?;

        logs.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 在临时目录建库并跑完迁移，尽量贴近真实环境
    fn temp_db(tag: &str) -> (PathBuf, Database) {
        let dir = std::env::temp_dir().join(format!("spaceshift-db-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let db = Database::new(&dir).unwrap();

        (dir, db)
    }

    /// 重复导入同一个工作区不能插出两条
    #[test]
    fn ensure_workspace_is_idempotent() {
        let (dir, db) = temp_db("ws");

        let first = db
            .ensure_workspace("ws-1", "日常", Some("📁"), Some("#409eff"), None)
            .unwrap();
        let second = db
            .ensure_workspace("ws-1", "日常", Some("📁"), Some("#409eff"), None)
            .unwrap();

        assert_eq!(first, "ws-1");
        assert_eq!(second, "ws-1");
        assert_eq!(db.get_all_workspaces().unwrap().len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 导入要连 id、排序、模板标记、启动参数一起原样写入并读回
    #[test]
    fn import_profile_keeps_every_field() {
        let (dir, db) = temp_db("import");
        db.ensure_workspace("ws-1", "日常", None, None, None).unwrap();

        let args = LaunchArgs {
            proxy: Some("http://127.0.0.1:7890".to_string()),
            ..LaunchArgs::default()
        };

        db.import_profile(
            "p-1",
            "导入的配置",
            "D:/SpaceShift/profiles/导入的配置",
            Some("C:/chrome.exe"),
            Some("https://example.com"),
            Some("data:image/png;base64,AAA"),
            Some("工作,测试"),
            Some(9222),
            true,
            true,
            Some(9333),
            Some("ws-1"),
            Some(&args),
            true,
            7,
        )
        .unwrap();

        let p = db
            .get_profile_by_id("p-1")
            .unwrap()
            .expect("导入的配置应该能读回来");

        assert_eq!(p.name, "导入的配置");
        assert_eq!(p.chrome_path.as_deref(), Some("C:/chrome.exe"));
        assert_eq!(p.homepage.as_deref(), Some("https://example.com"));
        assert_eq!(p.icon_base64.as_deref(), Some("data:image/png;base64,AAA"));
        assert_eq!(p.tags.as_deref(), Some("工作,测试"));
        assert_eq!(p.forward_port, Some(9222));
        assert!(p.disable_extensions);
        assert!(p.debugger_mode);
        assert_eq!(p.debug_port_config, Some(9333));
        assert_eq!(p.workspace_id.as_deref(), Some("ws-1"));
        assert_eq!(p.launch_args.proxy.as_deref(), Some("http://127.0.0.1:7890"));
        assert!(p.is_template);
        assert_eq!(p.sort_order, Some(7));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
