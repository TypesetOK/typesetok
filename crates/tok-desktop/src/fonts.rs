use crate::assets::tok_asset_font_url;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use tok_engine_service::{inspect_font_file, write_atomic};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFontEntry {
    pub file_name: String,
    pub path: String,
    pub asset_url: String,
    pub report: Value,
}

#[cfg(windows)]
pub fn list_system_fonts() -> Vec<String> {
    use std::ffi::c_void;
    use windows_sys::Win32::Graphics::Gdi::{
        EnumFontFamiliesExW, GetDC, ReleaseDC, DEFAULT_CHARSET, ENUMLOGFONTEXW, LOGFONTW,
        TEXTMETRICW,
    };

    unsafe extern "system" fn enum_font_proc(
        lpelfe: *const LOGFONTW,
        _lpntme: *const TEXTMETRICW,
        _font_type: u32,
        lparam: isize,
    ) -> i32 {
        if lpelfe.is_null() || lparam == 0 {
            return 1;
        }
        let set = &mut *(lparam as *mut BTreeSet<String>);
        let enum_lf = &*(lpelfe as *const ENUMLOGFONTEXW);
        let raw = &enum_lf.elfLogFont.lfFaceName;
        let len = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
        let name = String::from_utf16_lossy(&raw[..len]).trim().to_string();
        if !name.is_empty() && !name.starts_with('@') {
            set.insert(name);
        }
        1
    }

    let mut families = BTreeSet::new();
    unsafe {
        let hdc = GetDC(std::ptr::null_mut());
        if !hdc.is_null() {
            let mut lf: LOGFONTW = std::mem::zeroed();
            lf.lfCharSet = DEFAULT_CHARSET;
            let lparam = (&mut families as *mut BTreeSet<String>) as isize;
            EnumFontFamiliesExW(hdc, &lf, Some(enum_font_proc), lparam, 0);
            ReleaseDC(std::ptr::null_mut(), hdc);
        }
    }
    let _ = std::ptr::null::<c_void>();
    families.into_iter().collect()
}

#[cfg(not(windows))]
pub fn list_system_fonts() -> Vec<String> {
    Vec::new()
}

pub fn inspect_font(path: &Path) -> Result<Value, String> {
    if !path.is_file() {
        return Ok(json!({
            "valid": false,
            "error": "FileNotFound",
            "message": format!("Font file not found: {}", path.display())
        }));
    }
    inspect_font_file(path).map_err(|e| e.to_string())
}

pub fn install_font(source_path: &Path, fonts_dir: &Path) -> Result<Value, String> {
    if !source_path.is_file() {
        return Ok(json!({
            "success": false,
            "error": "FileNotFound",
            "message": format!("Font file not found: {}", source_path.display())
        }));
    }

    let inspection = inspect_font_file(source_path).map_err(|e| e.to_string())?;
    let valid = inspection
        .get("valid")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !valid {
        let mut obj = inspection.as_object().cloned().unwrap_or_default();
        obj.insert("success".to_string(), Value::Bool(false));
        return Ok(Value::Object(obj));
    }

    fs::create_dir_all(fonts_dir).map_err(|e| e.to_string())?;
    let raw_base = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("font.ttf");
    let sanitized: String = raw_base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let lower = sanitized.to_ascii_lowercase();
    let dest_name = if lower.ends_with(".ttf") || lower.ends_with(".otf") {
        sanitized
    } else {
        format!("{sanitized}.ttf")
    };
    let dest_path = fonts_dir.join(&dest_name);
    let bytes = fs::read(source_path).map_err(|e| e.to_string())?;
    write_atomic(&dest_path, &bytes).map_err(|e| e.to_string())?;

    let report = inspection.get("report").cloned().unwrap_or(Value::Null);
    Ok(json!({
        "success": true,
        "path": dest_path.to_string_lossy().into_owned(),
        "assetUrl": tok_asset_font_url(&dest_name),
        "report": report,
    }))
}

pub fn list_user_fonts(fonts_dir: &Path) -> Vec<UserFontEntry> {
    if !fonts_dir.is_dir() {
        return Vec::new();
    }
    let mut files: Vec<_> = fs::read_dir(fonts_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let p = e.path();
            p.is_file()
                && p.extension().and_then(|s| s.to_str()).is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf")
                })
        })
        .collect();
    files.sort_by_key(|e| e.file_name());

    let mut out = Vec::new();
    for entry in files {
        let full = entry.path();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if let Ok(parsed) = inspect_font_file(&full) {
            let valid = parsed
                .get("valid")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if valid {
                if let Some(report) = parsed.get("report").cloned() {
                    out.push(UserFontEntry {
                        asset_url: tok_asset_font_url(&file_name),
                        file_name,
                        path: full.to_string_lossy().into_owned(),
                        report,
                    });
                }
            }
        }
    }
    out
}
