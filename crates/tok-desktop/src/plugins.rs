use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tok_engine_service::write_atomic;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub main: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_tok_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub dir_path: String,
    pub source_type: &'static str,
    pub compiled_code: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub safe_mode_blocked: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSystemStatus {
    pub is_safe_mode: bool,
    pub is_safer_active: bool,
    pub user_plugins_dir: String,
    pub builtin_plugins_dir: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedPluginState {
    #[serde(default)]
    safe_mode: Option<bool>,
    #[serde(default)]
    states: BTreeMap<String, bool>,
}

pub fn is_safer_environment() -> bool {
    #[cfg(windows)]
    {
        for candidate in [
            r"C:\Program Files\safer",
            r"C:\Program Files (x86)\safer",
            r"C:\safer",
        ] {
            if Path::new(candidate).exists() {
                return true;
            }
        }
    }
    env::args().any(|a| a == "--safer") || env::var("SAFER_MODE").as_deref() == Ok("1")
}

pub fn is_safe_mode_requested() -> bool {
    env::args().any(|a| a == "--safe-mode" || a == "--disable-plugins")
        || env::var("TOK_SAFE_MODE").as_deref() == Ok("1")
}

/// Lightweight TypeScript-to-JavaScript type stripper for user plugins,
/// eliminating the 11.15 MB `esbuild.exe` runtime dependency.
pub fn strip_typescript_types(ts_source: &str) -> String {
    let mut out = ts_source.to_string();

    // Remove `import type ...;` lines
    let re_import_type = Regex::new(r"(?m)^\s*import\s+type\s+[^;]+;\s*$").unwrap();
    out = re_import_type.replace_all(&out, "").into_owned();

    // Remove `export type ...;` and `export interface ... { ... }`
    let re_type_alias = Regex::new(r"(?m)^\s*(?:export\s+)?type\s+[A-Za-z_$][\w$]*\s*(?:<[^>]*>)?\s*=\s*[^;]+;\s*$").unwrap();
    out = re_type_alias.replace_all(&out, "").into_owned();

    let re_interface = Regex::new(r"(?s)(?:export\s+)?interface\s+[A-Za-z_$][\w$]*(?:\s*<[^>]*>)?(?:\s+extends\s+[^{]+)?\s*\{[^{}]*\}").unwrap();
    out = re_interface.replace_all(&out, "").into_owned();

    // Strip `export ` keyword if plugin uses `export function activate` / `export function deactivate`
    let re_export_fn = Regex::new(r"(?m)^\s*export\s+(async\s+)?function\s+").unwrap();
    out = re_export_fn.replace_all(&out, "${1}function ").into_owned();

    // Strip parameter & return type annotations on function signatures and arrow functions:
    // `(a: string, b?: number): void` -> `(a, b)`
    let re_param_type = Regex::new(r"([A-Za-z_$][\w$]*)\s*\??:\s*[A-Za-z_$][\w$.<>|\[\]\s]*(?:[,)])").unwrap();
    for _ in 0..4 {
        let next = re_param_type
            .replace_all(&out, |caps: &regex::Captures<'_>| {
                let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let tail = full.chars().last().unwrap_or(')');
                format!("{name}{tail}")
            })
            .into_owned();
        if next == out {
            break;
        }
        out = next;
    }

    // Strip return types before `{` or `=>`
    let re_return_type = Regex::new(r"\)\s*:\s*[A-Za-z_$][\w$.<>|\[\]]*(\s*(?:\{|=>))").unwrap();
    out = re_return_type.replace_all(&out, ")$1").into_owned();

    // Strip `as Type` assertions
    let re_as_type = Regex::new(r"\s+as\s+(?:const|[A-Za-z_$][\w$.<>|\[\]]*)").unwrap();
    out = re_as_type.replace_all(&out, "").into_owned();

    out
}

fn resolve_safe_plugin_entry(plugin_dir: &Path, main_rel: &str) -> Result<PathBuf, String> {
    let rel = Path::new(main_rel);
    if rel.is_absolute()
        || main_rel.contains("..")
        || main_rel.contains('\0')
    {
        return Err("Unsafe plugin entry path".to_string());
    }
    let canon_dir = fs::canonicalize(plugin_dir).map_err(|e| e.to_string())?;
    let entry = canon_dir.join(rel);
    let canon_entry = fs::canonicalize(&entry).map_err(|e| e.to_string())?;
    if !canon_entry.starts_with(&canon_dir) || !canon_entry.is_file() {
        return Err("Plugin entry escapes plugin directory".to_string());
    }
    Ok(canon_entry)
}

pub struct PluginSystem {
    user_plugins_dir: PathBuf,
    builtin_plugins_dir: PathBuf,
    state_file: PathBuf,
    persisted: PersistedPluginState,
    safer_active: bool,
    cli_safe_mode: bool,
}

impl PluginSystem {
    pub fn load(user_data_dir: &Path) -> Self {
        let user_plugins_dir = user_data_dir.join("plugins");
        let _ = fs::create_dir_all(&user_plugins_dir);
        let state_file = user_data_dir.join("plugins_state.json");
        let persisted = fs::read_to_string(&state_file)
            .ok()
            .and_then(|raw| serde_json::from_str::<PersistedPluginState>(&raw).ok())
            .unwrap_or_default();

        Self {
            user_plugins_dir,
            builtin_plugins_dir: PathBuf::new(),
            state_file,
            persisted,
            safer_active: is_safer_environment(),
            cli_safe_mode: is_safe_mode_requested(),
        }
    }

    pub fn user_plugins_dir(&self) -> &Path {
        &self.user_plugins_dir
    }

    pub fn is_safe_mode(&self) -> bool {
        self.cli_safe_mode || self.persisted.safe_mode.unwrap_or(false)
    }

    pub fn get_status(&self) -> PluginSystemStatus {
        PluginSystemStatus {
            is_safe_mode: self.is_safe_mode(),
            is_safer_active: self.safer_active,
            user_plugins_dir: self.user_plugins_dir.to_string_lossy().into_owned(),
            builtin_plugins_dir: self.builtin_plugins_dir.to_string_lossy().into_owned(),
        }
    }

    pub fn set_safe_mode(&mut self, enabled: bool) -> Result<bool, String> {
        self.persisted.safe_mode = Some(enabled);
        self.save_state()?;
        Ok(true)
    }

    pub fn toggle_plugin(&mut self, plugin_id: &str, enabled: bool) -> Result<bool, String> {
        if plugin_id.trim().is_empty() {
            return Ok(false);
        }
        self.persisted.states.insert(plugin_id.to_string(), enabled);
        self.save_state()?;
        Ok(true)
    }

    fn save_state(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.persisted).map_err(|e| e.to_string())?;
        write_atomic(&self.state_file, json.as_bytes()).map_err(|e| e.to_string())
    }

    pub fn discover_plugins(&self) -> Vec<LoadedPlugin> {
        let safe_mode = self.is_safe_mode();
        let mut out = Vec::new();
        let Ok(entries) = fs::read_dir(&self.user_plugins_dir) else {
            return out;
        };
        let mut dirs: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();

        for dir in dirs {
            let manifest_path = dir.join("plugin.json");
            if !manifest_path.is_file() {
                continue;
            }
            let Ok(raw_manifest) = fs::read_to_string(&manifest_path) else {
                continue;
            };
            let Ok(manifest) = serde_json::from_str::<PluginManifest>(&raw_manifest) else {
                continue;
            };
            if manifest.id.trim().is_empty() || manifest.main.trim().is_empty() {
                continue;
            }

            let enabled = self
                .persisted
                .states
                .get(&manifest.id)
                .copied()
                .or(manifest.enabled)
                .unwrap_or(true);

            let is_ts = manifest.main.to_ascii_lowercase().ends_with(".ts");
            let source_type = if is_ts { "ts" } else { "js" };

            if safe_mode || self.safer_active {
                out.push(LoadedPlugin {
                    manifest,
                    dir_path: dir.to_string_lossy().into_owned(),
                    source_type,
                    compiled_code: String::new(),
                    enabled,
                    error: None,
                    safe_mode_blocked: true,
                });
                continue;
            }

            let (compiled_code, error) = match resolve_safe_plugin_entry(&dir, &manifest.main) {
                Ok(entry_path) => match fs::read_to_string(&entry_path) {
                    Ok(code) => {
                        let js = if is_ts {
                            strip_typescript_types(&code)
                        } else {
                            code
                        };
                        (js, None)
                    }
                    Err(err) => (String::new(), Some(err.to_string())),
                },
                Err(err) => (String::new(), Some(err)),
            };

            out.push(LoadedPlugin {
                manifest,
                dir_path: dir.to_string_lossy().into_owned(),
                source_type,
                compiled_code,
                enabled,
                error,
                safe_mode_blocked: false,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_typescript_types_handles_basic_plugin() {
        let ts = r#"
import type { TokPluginContext } from './types';
interface MyOpts { count: number; }
export function activate(tok: TokPluginContext, opts?: MyOpts): void {
  const x = 'hi' as string;
  tok.ui.showToast(x);
}
"#;
        let js = strip_typescript_types(ts);
        assert!(!js.contains("import type"));
        assert!(!js.contains("interface MyOpts"));
        assert!(!js.contains("export function"));
        assert!(js.contains("function activate(tok, opts) {"));
        assert!(js.contains("const x = 'hi';"));
    }

    #[test]
    fn resolve_safe_plugin_entry_blocks_traversal() {
        let dir = env::temp_dir().join(format!("tok_plugin_sec_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("index.js"), "function activate(){}").unwrap();

        assert!(resolve_safe_plugin_entry(&dir, "index.js").is_ok());
        assert!(resolve_safe_plugin_entry(&dir, "../outside.js").is_err());

        let _ = fs::remove_dir_all(&dir);
    }
}
