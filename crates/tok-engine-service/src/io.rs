use crate::error::EngineServiceError;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Atomically writes `bytes` to `output` using a sibling temp file, fsync, and rename.
pub fn write_atomic(output: &Path, bytes: &[u8]) -> Result<(), EngineServiceError> {
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = output
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("output");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = parent.join(format!(
        ".{}.tmp.{}.{}",
        file_name,
        std::process::id(),
        nanos
    ));

    let write_res = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(err) = write_res {
        let _ = fs::remove_file(&tmp_path);
        return Err(err.into());
    }

    if let Err(err) = fs::rename(&tmp_path, output) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err.into());
    }

    Ok(())
}

/// Sanitizes an asset filename to ASCII alphanumerics, `-`, `_`, and `.`.
pub fn sanitize_asset_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Lowercase hex encoding helper without extra crate dependencies.
pub fn format_hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}
