use crate::state::AppState;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::http::{header::CONTENT_TYPE, Request, Response, StatusCode};
use tauri::{AppHandle, Manager, UriSchemeContext, UriSchemeResponder};

/// Generates the platform-appropriate `tok-asset://` URL for an installed user font.
pub fn tok_asset_font_url(file_name: &str) -> String {
    let encoded: String = file_name
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    if cfg!(windows) {
        format!("http://tok-asset.localhost/fonts/{encoded}")
    } else {
        format!("tok-asset://localhost/fonts/{encoded}")
    }
}

fn is_allowed_origin(origin: &str) -> bool {
    let lower = origin.trim().to_ascii_lowercase();
    lower == "http://tauri.localhost"
        || lower == "https://tauri.localhost"
        || lower == "tauri://localhost"
        || lower == "null"
        || lower.starts_with("http://localhost:")
        || lower == "http://localhost"
        || lower.starts_with("http://127.0.0.1:")
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            let val = u8::from_str_radix(hex, 16).ok()?;
            out.push(val);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn sanitize_font_filename(raw_path: &str) -> Option<String> {
    let stripped = raw_path.strip_prefix("/fonts/")?;
    if stripped.is_empty()
        || stripped.contains('/')
        || stripped.contains('\\')
        || stripped.contains("..")
        || stripped.contains('\0')
    {
        return None;
    }
    let decoded = percent_decode(stripped)?;
    if decoded.is_empty()
        || decoded.contains('/')
        || decoded.contains('\\')
        || decoded.contains("..")
        || decoded.contains('\0')
    {
        return None;
    }
    let lower = decoded.to_ascii_lowercase();
    if !(lower.ends_with(".ttf")
        || lower.ends_with(".otf")
        || lower.ends_with(".woff")
        || lower.ends_with(".woff2"))
    {
        return None;
    }
    Some(decoded)
}

fn font_mime_type(file_name: &str) -> &'static str {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".otf") {
        "font/otf"
    } else if lower.ends_with(".woff") {
        "font/woff"
    } else if lower.ends_with(".woff2") {
        "font/woff2"
    } else {
        "font/ttf"
    }
}

fn resolve_authorized_font_file(fonts_dir: &Path, uri_path: &str) -> Result<(PathBuf, &'static str), StatusCode> {
    let file_name = sanitize_font_filename(uri_path).ok_or(StatusCode::NOT_FOUND)?;
    let canon_dir = fs::canonicalize(fonts_dir).map_err(|_| StatusCode::NOT_FOUND)?;
    let candidate = canon_dir.join(&file_name);
    let canon_file = fs::canonicalize(&candidate).map_err(|_| StatusCode::NOT_FOUND)?;
    if !canon_file.starts_with(&canon_dir) || !canon_file.is_file() {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok((canon_file, font_mime_type(&file_name)))
}

fn empty_response(status: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .body(Vec::new())
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

pub fn handle_tok_asset_protocol<R: tauri::Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    if let Some(origin) = request
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
    {
        if !is_allowed_origin(origin) {
            responder.respond(empty_response(StatusCode::FORBIDDEN));
            return;
        }
    }

    let app: AppHandle<R> = ctx.app_handle().clone();
    let uri_path = request.uri().path().to_string();

    tauri::async_runtime::spawn_blocking(move || {
        let Some(state) = app.try_state::<AppState>() else {
            responder.respond(empty_response(StatusCode::SERVICE_UNAVAILABLE));
            return;
        };
        let fonts_dir = state.fonts_dir.clone();
        match resolve_authorized_font_file(&fonts_dir, &uri_path) {
            Ok((file_path, mime)) => match fs::read(&file_path) {
                Ok(bytes) => {
                    let resp = Response::builder()
                        .status(StatusCode::OK)
                        .header(CONTENT_TYPE, mime)
                        .header("Cache-Control", "private, max-age=3600")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(bytes)
                        .unwrap_or_else(|_| empty_response(StatusCode::INTERNAL_SERVER_ERROR));
                    responder.respond(resp);
                }
                Err(_) => responder.respond(empty_response(StatusCode::NOT_FOUND)),
            },
            Err(status) => responder.respond(empty_response(status)),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn sanitize_font_filename_blocks_traversal_and_bad_extensions() {
        assert_eq!(
            sanitize_font_filename("/fonts/David.ttf").as_deref(),
            Some("David.ttf")
        );
        assert_eq!(
            sanitize_font_filename("/fonts/My%20Font.otf").as_deref(),
            Some("My Font.otf")
        );
        assert!(sanitize_font_filename("/fonts/../secret.ttf").is_none());
        assert!(sanitize_font_filename("/fonts/%2e%2e/secret.ttf").is_none());
        assert!(sanitize_font_filename("/fonts/sub/David.ttf").is_none());
        assert!(sanitize_font_filename("/fonts/script.js").is_none());
        assert!(sanitize_font_filename("/other/David.ttf").is_none());
    }

    #[test]
    fn resolve_authorized_font_file_only_serves_files_inside_fonts_dir() {
        let dir = env::temp_dir().join(format!("tok_assets_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let fonts_dir = dir.join("fonts");
        fs::create_dir_all(&fonts_dir).unwrap();
        let font_file = fonts_dir.join("TestFont.ttf");
        fs::write(&font_file, b"ttf-bytes").unwrap();

        let ok = resolve_authorized_font_file(&fonts_dir, "/fonts/TestFont.ttf");
        assert!(ok.is_ok());

        let missing = resolve_authorized_font_file(&fonts_dir, "/fonts/Missing.ttf");
        assert_eq!(missing.unwrap_err(), StatusCode::NOT_FOUND);

        let traversal = resolve_authorized_font_file(&fonts_dir, "/fonts/..%2Fsecret.ttf");
        assert_eq!(traversal.unwrap_err(), StatusCode::NOT_FOUND);

        let _ = fs::remove_dir_all(&dir);
    }
}
