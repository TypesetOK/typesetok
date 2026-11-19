use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tok_engine_service::{write_atomic, WorkspaceSession};

const MAX_SETTING_KEY_LEN: usize = 128;
const MAX_SETTING_VAL_LEN: usize = 2 * 1024 * 1024;
const FORBIDDEN_SETTING_KEYS: &[&str] = &["__proto__", "constructor", "prototype"];

#[derive(Debug, Clone, Default)]
pub struct InstallRegistryInfo {
    pub language: Option<String>,
    pub projects_dir: Option<String>,
}

pub fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn trace_milestone(enabled: bool, label: &str, epoch_ms: Option<u64>) {
    if !enabled {
        return;
    }
    let ts = epoch_ms.unwrap_or_else(now_epoch_ms);
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "[TOK-TRACE] {label} {ts}");
    let _ = out.flush();
}

pub fn trace_memory(enabled: bool, label: &str) {
    if !enabled {
        return;
    }
    if let Some(bytes) = current_process_working_set_bytes() {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "[TOK-MEM] {label} {bytes}");
        let _ = out.flush();
    }
}

#[cfg(windows)]
fn current_process_working_set_bytes() -> Option<u64> {
    use std::mem::size_of;
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    let ok = unsafe {
        K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
    };
    if ok != 0 {
        Some(counters.WorkingSetSize as u64)
    } else {
        None
    }
}

#[cfg(not(windows))]
fn current_process_working_set_bytes() -> Option<u64> {
    None
}

#[cfg(windows)]
pub fn read_install_registry() -> InstallRegistryInfo {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let mut info = InstallRegistryInfo::default();
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let hk = RegKey::predef(root);
        if let Ok(sub) = hk.open_subkey("Software\\TypesetOK") {
            if info.language.is_none() {
                if let Ok(lang) = sub.get_value::<String, _>("Language") {
                    let trimmed = lang.trim().to_ascii_lowercase();
                    if trimmed == "he" || trimmed == "en" {
                        info.language = Some(trimmed);
                    }
                }
            }
            if info.projects_dir.is_none() {
                if let Ok(dir) = sub.get_value::<String, _>("ProjectsDir") {
                    let trimmed = dir.trim().to_string();
                    if !trimmed.is_empty() {
                        info.projects_dir = Some(trimmed);
                    }
                }
            }
        }
    }
    info
}

#[cfg(not(windows))]
pub fn read_install_registry() -> InstallRegistryInfo {
    InstallRegistryInfo::default()
}

pub fn resolve_user_data_dir(app_data_fallback: Option<PathBuf>) -> PathBuf {
    if let Ok(override_dir) = env::var("TOK_USER_DATA_DIR") {
        let trimmed = override_dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if let Ok(appdata) = env::var("APPDATA") {
        let trimmed = appdata.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed).join("TypesetOK");
        }
    }
    app_data_fallback.unwrap_or_else(|| env::temp_dir().join("TypesetOK"))
}

pub fn resolve_projects_dir(reg_dir: Option<&str>, documents_dir: Option<PathBuf>) -> PathBuf {
    if let Some(dir_str) = reg_dir {
        let p = PathBuf::from(dir_str);
        if fs::create_dir_all(&p).is_ok() && p.is_dir() {
            return p;
        }
    }
    let default_dir = documents_dir
        .unwrap_or_else(|| env::temp_dir())
        .join("TypesetOK");
    let _ = fs::create_dir_all(&default_dir);
    default_dir
}

pub fn is_valid_setting_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_SETTING_KEY_LEN
        && !FORBIDDEN_SETTING_KEYS.contains(&key)
}

pub fn is_valid_setting_value(val: &str) -> bool {
    val.len() <= MAX_SETTING_VAL_LEN
}

pub struct SettingsStore {
    path: PathBuf,
    map: BTreeMap<String, String>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let mut map = BTreeMap::new();
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(serde_json::Value::Object(obj)) = serde_json::from_str(&raw) {
                for (k, v) in obj {
                    if let serde_json::Value::String(s) = v {
                        if is_valid_setting_key(&k) && is_valid_setting_value(&s) {
                            map.insert(k, s);
                        }
                    }
                }
            }
        }
        Self { path, map }
    }

    pub fn snapshot(&self) -> BTreeMap<String, String> {
        self.map.clone()
    }

    pub fn set(&mut self, key: String, value: String) -> Result<(), String> {
        if !is_valid_setting_key(&key) {
            return Err("Invalid setting key".to_string());
        }
        if !is_valid_setting_value(&value) {
            return Err("Invalid setting value".to_string());
        }
        self.map.insert(key, value);
        self.persist()
    }

    pub fn set_many(&mut self, entries: BTreeMap<String, String>) -> Result<(), String> {
        for (k, v) in &entries {
            if !is_valid_setting_key(k) {
                return Err(format!("Invalid setting key: {k}"));
            }
            if !is_valid_setting_value(v) {
                return Err(format!("Invalid setting value for key: {k}"));
            }
        }
        for (k, v) in entries {
            self.map.insert(k, v);
        }
        self.persist()
    }

    pub fn remove(&mut self, key: &str) -> Result<(), String> {
        if !is_valid_setting_key(key) {
            return Err("Invalid setting key".to_string());
        }
        if self.map.remove(key).is_some() {
            self.persist()?;
        }
        Ok(())
    }

    fn persist(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.map).map_err(|e| e.to_string())?;
        write_atomic(&self.path, json.as_bytes()).map_err(|e| e.to_string())
    }
}

pub struct PathPolicy {
    projects_dir: PathBuf,
    authorized_paths: HashSet<PathBuf>,
    authorized_dirs: HashSet<PathBuf>,
}

fn normalize_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| {
        if let Some(parent) = path.parent() {
            if let Ok(canon_parent) = fs::canonicalize(parent) {
                if let Some(name) = path.file_name() {
                    return canon_parent.join(name);
                }
            }
        }
        path.to_path_buf()
    })
}

impl PathPolicy {
    pub fn new(projects_dir: PathBuf) -> Self {
        Self {
            projects_dir: normalize_path(&projects_dir),
            authorized_paths: HashSet::new(),
            authorized_dirs: HashSet::new(),
        }
    }

    pub fn projects_dir(&self) -> &Path {
        &self.projects_dir
    }

    pub fn authorize_path(&mut self, path: &Path) {
        let norm = normalize_path(path);
        if let Some(parent) = norm.parent() {
            self.authorized_dirs.insert(parent.to_path_buf());
        }
        self.authorized_paths.insert(norm);
    }

    pub fn assert_authorized(&self, path: &Path) -> Result<PathBuf, String> {
        let norm = normalize_path(path);
        if norm.starts_with(&self.projects_dir) || self.authorized_paths.contains(&norm) {
            return Ok(norm);
        }
        if let Some(parent) = norm.parent() {
            if self.authorized_dirs.contains(parent) {
                return Ok(norm);
            }
        }
        Err(format!("Access denied for path: {}", path.display()))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPayload {
    pub platform: &'static str,
    pub install_language: Option<String>,
    pub startup_file: Option<String>,
    pub settings: BTreeMap<String, String>,
    pub capabilities: Vec<&'static str>,
}

pub const SUPPORTED_CAPABILITIES: &[&str] = &[
    "renderPdf",
    "renderHtml",
    "getRecentLogs",
    "getPlugins",
    "getPluginSystemStatus",
    "saveDocument",
    "openDocument",
    "getProjectsDir",
    "saveDraftRecovery",
    "loadDraftRecovery",
    "clearDraftRecovery",
    "typesetDocument",
    "runPreflight",
    "listUserFonts",
    "listSystemFonts",
    "openExternal",
    "getAppInfo",
    "showSaveDialog",
    "showOpenDialog",
    "showImportDialog",
    "getStartupFile",
    "closeWindow",
    "minimizeWindow",
    "maximizeWindow",
    "isMaximized",
    "quitApp",
    "readSettings",
    "setSetting",
    "setSettings",
    "removeSetting",
];

pub struct AppState {
    pub session: Arc<Mutex<WorkspaceSession>>,
    pub settings: Mutex<SettingsStore>,
    pub paths: Mutex<PathPolicy>,
    pub startup_file: Option<String>,
    pub user_data_dir: PathBuf,
    pub fonts_dir: PathBuf,
    pub draft_recovery_path: PathBuf,
    pub trace_mode: Option<String>,
}

impl AppState {
    pub fn trace_enabled(&self) -> bool {
        self.trace_mode.is_some()
    }

    pub fn trace_exit(&self) -> bool {
        self.trace_mode.as_deref() == Some("exit")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_store_roundtrip_and_validation() {
        let dir = env::temp_dir().join(format!("tok_desktop_settings_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        let mut store = SettingsStore::load(path.clone());
        store
            .set("tok_lang".to_string(), "he".to_string())
            .unwrap();
        assert!(store
            .set("__proto__".to_string(), "bad".to_string())
            .is_err());

        let reloaded = SettingsStore::load(path);
        assert_eq!(reloaded.snapshot().get("tok_lang").map(String::as_str), Some("he"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn path_policy_enforces_projects_dir_and_dialog_authorization() {
        let dir = env::temp_dir().join(format!("tok_desktop_paths_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let projects = dir.join("projects");
        let outside = dir.join("outside");
        fs::create_dir_all(&projects).unwrap();
        fs::create_dir_all(&outside).unwrap();

        let mut policy = PathPolicy::new(projects.clone());
        let inside_file = projects.join("book.tok");
        assert!(policy.assert_authorized(&inside_file).is_ok());

        let outside_file = outside.join("secret.tok");
        assert!(policy.assert_authorized(&outside_file).is_err());

        policy.authorize_path(&outside_file);
        assert!(policy.assert_authorized(&outside_file).is_ok());

        let _ = fs::remove_dir_all(&dir);
    }
}
