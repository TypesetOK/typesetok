use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use futures_util::StreamExt;
use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Runtime};

const REPO_OWNER: &str = "TypesetOK";
const REPO_NAME: &str = "typesetok";
const REQUEST_TIMEOUT_SECS: u64 = 8;
const MAX_INSTALLER_SIZE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterSslDiagnosis {
    pub is_filter_error: bool,
    pub filter_type: &'static str,
    pub filter_name_hebrew: &'static str,
    pub error_message_hebrew: &'static str,
    pub recommended_action_hebrew: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadProgress {
    pub percent: u32,
    pub transferred_bytes: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub eta_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadUpdateResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyUpdateResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineUpdateCandidate {
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub is_newer: bool,
    pub is_current: bool,
    pub is_recommended: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_letter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_present: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_description_hebrew: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_name: String,
    pub release_notes: String,
    pub release_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn releases_page() -> String {
    format!("https://github.com/{REPO_OWNER}/{REPO_NAME}/releases")
}

pub fn is_safe_external_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

pub fn compare_versions(v1: &str, v2: &str) -> i32 {
    let parse = |s: &str| -> Vec<u64> {
        s.trim()
            .trim_start_matches(['v', 'V'])
            .split('.')
            .map(|part| {
                let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
                digits.parse::<u64>().unwrap_or(0)
            })
            .collect()
    };
    let p1 = parse(v1);
    let p2 = parse(v2);
    let len = p1.len().max(p2.len());
    for i in 0..len {
        let a = *p1.get(i).unwrap_or(&0);
        let b = *p2.get(i).unwrap_or(&0);
        if a > b {
            return 1;
        }
        if a < b {
            return -1;
        }
    }
    0
}

pub fn diagnose_filter_ssl_error(err_msg: &str) -> FilterSslDiagnosis {
    let lower = err_msg.to_ascii_lowercase();
    if lower.contains("netfree") || err_msg.contains("נטפרי") {
        return FilterSslDiagnosis {
            is_filter_error: true,
            filter_type: "NetFree",
            filter_name_hebrew: "נטפרי (NetFree)",
            error_message_hebrew:
                "זוהתה חסימה או אי-התאמת תעודת אבטחה של סינון נטפרי בבדיקת עדכונים.",
            recommended_action_hebrew:
                "ודאו שתעודת האבטחה של נטפרי מותקנת כראוי במחשב (תוכנת התקנת תעודות של נטפרי), או הורידו את העדכון ישירות מהדפדפן.",
        };
    }
    if lower.contains("rimon") || err_msg.contains("רימון") {
        return FilterSslDiagnosis {
            is_filter_error: true,
            filter_type: "Rimon",
            filter_name_hebrew: "אינטרנט רימון",
            error_message_hebrew:
                "זוהתה אי-התאמת תעודת אבטחה של אינטרנט רימון בבדיקת עדכונים.",
            recommended_action_hebrew:
                "ודאו שתעודת האבטחה של רימון מותקנת במחשב, או גשו להורדה ידנית מדף השחרורים ב-GitHub.",
        };
    }
    if lower.contains("etrog") || err_msg.contains("אתרוג") {
        return FilterSslDiagnosis {
            is_filter_error: true,
            filter_type: "Etrog",
            filter_name_hebrew: "אינטרנט אתרוג",
            error_message_hebrew:
                "זוהתה אי-התאמת תעודת אבטחה של אינטרנט אתרוג בבדיקת עדכונים.",
            recommended_action_hebrew:
                "ודאו שתעודת האבטחה של אתרוג מותקנת במחשב, או הורידו את העדכון דרך הדפדפן.",
        };
    }
    let is_ssl = lower.contains("certificate")
        || lower.contains("ssl")
        || lower.contains("tls")
        || lower.contains("schannel")
        || lower.contains("self signed")
        || lower.contains("authority");
    if is_ssl {
        return FilterSslDiagnosis {
            is_filter_error: true,
            filter_type: "GenericProxy",
            filter_name_hebrew: "סינון אינטרנט / שרת פרוקסי מאובטח",
            error_message_hebrew:
                "שגיאת אבטחת תעודת SSL מול שרת העדכונים (סינון אתרים פעיל / תעודה מקומית).",
            recommended_action_hebrew:
                "בדקו שתעודת השורש של ספק הסינון מותקנת במאגר התעודות של Windows, או הורידו עדכון אופליין.",
        };
    }
    FilterSslDiagnosis {
        is_filter_error: false,
        filter_type: "Unknown",
        filter_name_hebrew: "שגיאת רשת",
        error_message_hebrew: "בדיקת העדכונים נכשלה עקב בעיית תקשורת או היעדר חיבור לאינטרנט.",
        recommended_action_hebrew: "בדקו את החיבור לאינטרנט או השתמשו בעדכון אופליין מכונן USB.",
    }
}

fn build_native_tls_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .use_native_tls()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent(format!("TypesetOK/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())
}

pub async fn check_for_updates(current_version: &str) -> UpdateCheckResult {
    let api_url = format!("https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest");
    let client = match build_native_tls_client() {
        Ok(c) => c,
        Err(e) => {
            return UpdateCheckResult {
                has_update: false,
                current_version: current_version.to_string(),
                latest_version: current_version.to_string(),
                release_name: String::new(),
                release_notes: String::new(),
                release_url: releases_page(),
                download_url: None,
                published_at: None,
                error: Some(e),
            };
        }
    };

    let resp = match client
        .get(&api_url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let msg = e.to_string();
            let diag = diagnose_filter_ssl_error(&msg);
            let final_err = if diag.is_filter_error {
                format!("{} {}", diag.error_message_hebrew, diag.recommended_action_hebrew)
            } else {
                msg
            };
            return UpdateCheckResult {
                has_update: false,
                current_version: current_version.to_string(),
                latest_version: current_version.to_string(),
                release_name: String::new(),
                release_notes: String::new(),
                release_url: releases_page(),
                download_url: None,
                published_at: None,
                error: Some(final_err),
            };
        }
    };

    if !resp.status().is_success() {
        return UpdateCheckResult {
            has_update: false,
            current_version: current_version.to_string(),
            latest_version: current_version.to_string(),
            release_name: String::new(),
            release_notes: String::new(),
            release_url: releases_page(),
            download_url: None,
            published_at: None,
            error: Some(format!("HTTP {}", resp.status().as_u16())),
        };
    }

    let json: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return UpdateCheckResult {
                has_update: false,
                current_version: current_version.to_string(),
                latest_version: current_version.to_string(),
                release_name: String::new(),
                release_notes: String::new(),
                release_url: releases_page(),
                download_url: None,
                published_at: None,
                error: Some(e.to_string()),
            };
        }
    };

    let tag = json
        .get("tag_name")
        .or_else(|| json.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let latest_version = tag.trim().trim_start_matches(['v', 'V']).to_string();
    let has_update = !latest_version.is_empty() && compare_versions(&latest_version, current_version) > 0;
    let release_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .filter(|u| is_safe_external_url(u))
        .map(|s| s.to_string())
        .unwrap_or_else(releases_page);

    let mut download_url = None;
    if let Some(assets) = json.get("assets").and_then(|v| v.as_array()) {
        for asset in assets {
            let name = asset
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let url = asset
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if is_safe_external_url(url) && (name.ends_with(".exe") || name.ends_with(".msi")) {
                download_url = Some(url.to_string());
                break;
            }
        }
    }

    UpdateCheckResult {
        has_update,
        current_version: current_version.to_string(),
        latest_version: if latest_version.is_empty() {
            current_version.to_string()
        } else {
            latest_version
        },
        release_name: json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(tag)
            .to_string(),
        release_notes: json
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        release_url,
        download_url,
        published_at: json
            .get("published_at")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        error: None,
    }
}

pub fn verify_ed25519_file_signature(
    file_path: &Path,
    sig_bytes_raw: &[u8],
    pubkey_bytes: &[u8; 32],
) -> Result<bool, String> {
    let verifying_key = VerifyingKey::from_bytes(pubkey_bytes).map_err(|e| e.to_string())?;
    let sig_bytes: [u8; 64] = if sig_bytes_raw.len() == 64 {
        sig_bytes_raw
            .try_into()
            .map_err(|_| "Invalid signature length".to_string())?
    } else {
        let text = std::str::from_utf8(sig_bytes_raw)
            .map_err(|_| "Signature file is not valid UTF-8 or 64 raw bytes".to_string())?
            .trim();
        if text.len() == 128 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
            let mut arr = [0u8; 64];
            for i in 0..64 {
                arr[i] = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
                    .map_err(|e| e.to_string())?;
            }
            arr
        } else {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(text)
                .map_err(|e| e.to_string())?;
            decoded
                .try_into()
                .map_err(|_| "Decoded signature is not 64 bytes".to_string())?
        }
    };

    let signature = Signature::from_bytes(&sig_bytes);
    let data = fs::read(file_path).map_err(|e| e.to_string())?;
    Ok(verifying_key.verify(&data, &signature).is_ok())
}

pub async fn download_update<R: Runtime>(
    app: &AppHandle<R>,
    download_url: &str,
    target_dir: &Path,
) -> DownloadUpdateResult {
    if !is_safe_external_url(download_url) {
        return DownloadUpdateResult {
            success: false,
            file_path: None,
            file_size: None,
            sha256: None,
            signature_verified: None,
            error: Some("כתובת הורדה לא תקינה".to_string()),
        };
    }

    let client = match reqwest::Client::builder()
        .use_native_tls()
        .timeout(Duration::from_secs(300))
        .user_agent(format!("TypesetOK/{}", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return DownloadUpdateResult {
                success: false,
                file_path: None,
                file_size: None,
                sha256: None,
                signature_verified: None,
                error: Some(e.to_string()),
            };
        }
    };

    let resp = match client.get(download_url).send().await {
        Ok(r) => r,
        Err(e) => {
            let diag = diagnose_filter_ssl_error(&e.to_string());
            let err_msg = if diag.is_filter_error {
                format!("{} {}", diag.error_message_hebrew, diag.recommended_action_hebrew)
            } else {
                e.to_string()
            };
            return DownloadUpdateResult {
                success: false,
                file_path: None,
                file_size: None,
                sha256: None,
                signature_verified: None,
                error: Some(err_msg),
            };
        }
    };

    if !resp.status().is_success() {
        return DownloadUpdateResult {
            success: false,
            file_path: None,
            file_size: None,
            sha256: None,
            signature_verified: None,
            error: Some(format!("HTTP {}", resp.status().as_u16())),
        };
    }

    let total_bytes = resp.content_length().unwrap_or(0);
    if total_bytes > MAX_INSTALLER_SIZE_BYTES {
        return DownloadUpdateResult {
            success: false,
            file_path: None,
            file_size: None,
            sha256: None,
            signature_verified: None,
            error: Some("קובץ ההתקנה חורג מהגודל המרבי המותר".to_string()),
        };
    }

    let _ = fs::create_dir_all(target_dir);
    let raw_name = download_url
        .split('?')
        .next()
        .and_then(|s| s.rsplit('/').next())
        .filter(|s| !s.is_empty())
        .unwrap_or("TypesetOK-Setup.exe");
    let safe_name: String = raw_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let final_path = target_dir.join(&safe_name);
    let tmp_path = target_dir.join(format!(".{safe_name}.download.tmp"));

    let mut file = match File::create(&tmp_path) {
        Ok(f) => f,
        Err(e) => {
            return DownloadUpdateResult {
                success: false,
                file_path: None,
                file_size: None,
                sha256: None,
                signature_verified: None,
                error: Some(e.to_string()),
            };
        }
    };

    let mut hasher = Sha256::new();
    let mut transferred: u64 = 0;
    let start = Instant::now();
    let mut last_emit = Instant::now();
    let mut stream = resp.bytes_stream();

    while let Some(chunk_res) = stream.next().await {
        let chunk = match chunk_res {
            Ok(c) => c,
            Err(e) => {
                let _ = fs::remove_file(&tmp_path);
                return DownloadUpdateResult {
                    success: false,
                    file_path: None,
                    file_size: None,
                    sha256: None,
                    signature_verified: None,
                    error: Some(e.to_string()),
                };
            }
        };
        transferred = transferred.saturating_add(chunk.len() as u64);
        if transferred > MAX_INSTALLER_SIZE_BYTES {
            let _ = fs::remove_file(&tmp_path);
            return DownloadUpdateResult {
                success: false,
                file_path: None,
                file_size: None,
                sha256: None,
                signature_verified: None,
                error: Some("קובץ ההתקנה חורג מהגודל המרבי המותר".to_string()),
            };
        }
        if let Err(e) = file.write_all(&chunk) {
            let _ = fs::remove_file(&tmp_path);
            return DownloadUpdateResult {
                success: false,
                file_path: None,
                file_size: None,
                sha256: None,
                signature_verified: None,
                error: Some(e.to_string()),
            };
        }
        hasher.update(&chunk);

        if last_emit.elapsed() >= Duration::from_millis(120) {
            last_emit = Instant::now();
            let elapsed = start.elapsed().as_secs_f64().max(0.05);
            let speed = (transferred as f64 / elapsed) as u64;
            let percent = if total_bytes > 0 {
                ((transferred * 100) / total_bytes).min(100) as u32
            } else {
                0
            };
            let eta = if total_bytes > transferred && speed > 0 {
                (total_bytes - transferred) / speed
            } else {
                0
            };
            let _ = app.emit(
                "tok:update-download-progress",
                UpdateDownloadProgress {
                    percent,
                    transferred_bytes: transferred,
                    total_bytes,
                    speed_bytes_per_sec: speed,
                    eta_seconds: eta,
                },
            );
        }
    }

    let _ = file.flush();
    drop(file);

    // Verify PE magic header ("MZ") for .exe installers
    if safe_name.to_ascii_lowercase().ends_with(".exe") {
        let mut magic = [0u8; 2];
        let valid_pe = File::open(&tmp_path)
            .and_then(|mut f| f.read_exact(&mut magic))
            .is_ok()
            && &magic == b"MZ";
        if !valid_pe {
            let _ = fs::remove_file(&tmp_path);
            return DownloadUpdateResult {
                success: false,
                file_path: None,
                file_size: None,
                sha256: None,
                signature_verified: None,
                error: Some("הקובץ שהורד אינו קובץ התקנה תקין (PE/MZ)".to_string()),
            };
        }
    }

    if let Err(e) = fs::rename(&tmp_path, &final_path) {
        let _ = fs::remove_file(&tmp_path);
        return DownloadUpdateResult {
            success: false,
            file_path: None,
            file_size: None,
            sha256: None,
            signature_verified: None,
            error: Some(e.to_string()),
        };
    }

    let digest = hasher.finalize();
    let sha256_hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();

    let _ = app.emit(
        "tok:update-download-progress",
        UpdateDownloadProgress {
            percent: 100,
            transferred_bytes: transferred,
            total_bytes: total_bytes.max(transferred),
            speed_bytes_per_sec: 0,
            eta_seconds: 0,
        },
    );

    DownloadUpdateResult {
        success: true,
        file_path: Some(final_path.to_string_lossy().into_owned()),
        file_size: Some(transferred),
        sha256: Some(sha256_hex),
        signature_verified: None,
        error: None,
    }
}

fn extract_version_from_filename(name: &str) -> Option<String> {
    let re = Regex::new(r"v?(\d+\.\d+\.\d+)").ok()?;
    re.captures(name)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

pub fn inspect_offline_file(file_path: &Path, current_version: &str) -> Option<OfflineUpdateCandidate> {
    let meta = fs::metadata(file_path).ok()?;
    if !meta.is_file() {
        return None;
    }
    let file_name = file_path.file_name()?.to_string_lossy().into_owned();
    let lower = file_name.to_ascii_lowercase();
    if !(lower.ends_with(".exe") || lower.ends_with(".msi")) {
        return None;
    }
    if !lower.contains("typesetok") && !lower.contains("tok") {
        return None;
    }

    let version = extract_version_from_filename(&file_name);
    let cmp = version
        .as_deref()
        .map(|v| compare_versions(v, current_version))
        .unwrap_or(0);
    let is_newer = cmp > 0;
    let is_current = version.is_some() && cmp == 0;

    let status_description_hebrew = if is_newer {
        Some(format!("גרסה חדשה זמינה ({})", version.as_deref().unwrap_or("")))
    } else if is_current {
        Some("זהה לגרסה המותקנת".to_string())
    } else if version.is_some() {
        Some("גרסה ישנה מהמותקנת".to_string())
    } else {
        Some("קובץ התקנה אופליין".to_string())
    };

    let sig_path = PathBuf::from(format!("{}.sig", file_path.display()));
    let signature_present = Some(sig_path.is_file());

    let path_str = file_path.to_string_lossy().into_owned();
    let drive_letter = path_str
        .chars()
        .nth(1)
        .filter(|&c| c == ':')
        .and_then(|_| path_str.chars().next())
        .map(|c| c.to_ascii_uppercase().to_string());

    Some(OfflineUpdateCandidate {
        file_path: path_str,
        file_name,
        file_size: meta.len(),
        version,
        is_newer,
        is_current,
        is_recommended: is_newer,
        drive_letter,
        sha256: None,
        signature_present,
        status_description_hebrew,
    })
}

#[cfg(windows)]
fn removable_drive_roots() -> Vec<PathBuf> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    const DRIVE_REMOVABLE: u32 = 2;

    let mask = unsafe { GetLogicalDrives() };
    let mut roots = Vec::new();
    for i in 0..26u32 {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        let letter = (b'A' + i as u8) as char;
        // Skip A: and B: legacy floppy drives
        if letter == 'A' || letter == 'B' {
            continue;
        }
        let root_str = format!("{letter}:\\");
        let wide: Vec<u16> = root_str.encode_utf16().chain(std::iter::once(0)).collect();
        let drive_type = unsafe { GetDriveTypeW(wide.as_ptr()) };
        if drive_type == DRIVE_REMOVABLE {
            roots.push(PathBuf::from(root_str));
        }
    }
    roots
}

#[cfg(not(windows))]
fn removable_drive_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for base in ["/media", "/run/media", "/mnt", "/Volumes"] {
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    roots.push(entry.path());
                }
            }
        }
    }
    roots
}

pub fn scan_usb_drives(current_version: &str) -> Vec<OfflineUpdateCandidate> {
    let keywords = [
        "typesetok",
        "תוכנות",
        "הורדות",
        "עדכונים",
        "עימוד",
        "ספרים",
        "ספרי",
        "קודש",
        "installers",
        "updates",
        "software",
    ];

    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    for root in removable_drive_roots() {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let full = entry.path();
            if full.is_file() {
                if seen.insert(full.clone()) {
                    if let Some(c) = inspect_offline_file(&full, current_version) {
                        candidates.push(c);
                    }
                }
            } else if full.is_dir() {
                let lower = entry.file_name().to_string_lossy().to_ascii_lowercase();
                if keywords.iter().any(|kw| lower.contains(kw)) {
                    if let Ok(sub_entries) = fs::read_dir(&full) {
                        for sub in sub_entries.flatten() {
                            let sub_path = sub.path();
                            if sub_path.is_file() && seen.insert(sub_path.clone()) {
                                if let Some(c) = inspect_offline_file(&sub_path, current_version) {
                                    candidates.push(c);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    candidates.sort_by(|a, b| match (&a.version, &b.version) {
        (Some(va), Some(vb)) => compare_versions(vb, va).cmp(&0),
        _ => std::cmp::Ordering::Equal,
    });

    let mut marked = false;
    for cand in &mut candidates {
        if cand.is_newer && !marked {
            cand.is_recommended = true;
            marked = true;
        } else {
            cand.is_recommended = false;
        }
    }

    candidates
}

pub fn apply_installer(file_path: &Path, silent: bool) -> ApplyUpdateResult {
    if !file_path.is_file() {
        return ApplyUpdateResult {
            success: false,
            error: Some("קובץ ההתקנה לא נמצא".to_string()),
        };
    }

    let mut cmd = Command::new(file_path);
    if silent {
        cmd.arg("/VERYSILENT").arg("/SUPPRESSMSGBOXES").arg("/NORESTART");
    }
    match cmd.spawn() {
        Ok(_) => ApplyUpdateResult {
            success: true,
            error: None,
        },
        Err(e) => ApplyUpdateResult {
            success: false,
            error: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn compare_versions_orders_semver_correctly() {
        assert_eq!(compare_versions("0.37.0", "0.36.6"), 1);
        assert_eq!(compare_versions("v0.36.6", "0.36.6"), 0);
        assert_eq!(compare_versions("0.35.9", "0.36.6"), -1);
    }

    #[test]
    fn diagnose_filter_ssl_detects_netfree_rimon_etrog() {
        assert_eq!(
            diagnose_filter_ssl_error("NetFree root CA untrusted").filter_type,
            "NetFree"
        );
        assert_eq!(
            diagnose_filter_ssl_error("internet-rimon certificate error").filter_type,
            "Rimon"
        );
        assert_eq!(
            diagnose_filter_ssl_error("Etrog SSL proxy").filter_type,
            "Etrog"
        );
    }

    #[test]
    fn verify_ed25519_file_signature_validates_hex_signature() {
        // Deterministic test keypair from fixed seed
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key();
        use ed25519_dalek::Signer;

        let dir = env::temp_dir().join(format!("tok_updater_sig_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("installer.exe");
        let payload = b"MZ-test-installer-payload";
        fs::write(&file_path, payload).unwrap();

        let sig = signing_key.sign(payload);
        let hex_sig: String = sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect();

        assert!(
            verify_ed25519_file_signature(&file_path, hex_sig.as_bytes(), &verifying_key.to_bytes())
                .unwrap()
        );

        let mut bad_hex = hex_sig.clone();
        bad_hex.replace_range(0..2, if &bad_hex[0..2] == "00" { "ff" } else { "00" });
        assert!(
            !verify_ed25519_file_signature(&file_path, bad_hex.as_bytes(), &verifying_key.to_bytes())
                .unwrap()
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
