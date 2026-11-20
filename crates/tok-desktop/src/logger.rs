use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_RETENTION_DAYS: u64 = 14;

/// Daily rotating file logger for the Tauri desktop shell.
pub struct TokLogger {
    logs_dir: PathBuf,
    retention_days: AtomicU64,
    write_lock: Mutex<()>,
}

impl TokLogger {
    pub fn new(user_data_dir: &Path) -> Self {
        let logs_dir = user_data_dir.join("logs");
        let _ = fs::create_dir_all(&logs_dir);
        Self {
            logs_dir,
            retention_days: AtomicU64::new(DEFAULT_RETENTION_DAYS),
            write_lock: Mutex::new(()),
        }
    }

    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    pub fn set_retention_days(&self, days: u64) {
        if days > 0 {
            self.retention_days.store(days, Ordering::SeqCst);
        }
    }

    fn current_date_ymd() -> (i32, u32, u32, u32, u32, u32) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // Civil-from-days algorithm (UTC)
        let days = (secs / 86_400) as i64;
        let rem = secs % 86_400;
        let hour = (rem / 3600) as u32;
        let min = ((rem % 3600) / 60) as u32;
        let sec = (rem % 60) as u32;

        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = (z - era * 146_097) as u64;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = (yoe as i64) + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
        let year = (y + if m <= 2 { 1 } else { 0 }) as i32;
        (year, m, d, hour, min, sec)
    }

    fn today_log_file(&self) -> PathBuf {
        let (y, m, d, _, _, _) = Self::current_date_ymd();
        self.logs_dir.join(format!("tok-{y:04}-{m:02}-{d:02}.log"))
    }

    pub fn log(&self, level: &str, message: &str) {
        let (y, mo, d, h, mi, s) = Self::current_date_ymd();
        let line = format!("[{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}] [{level}] {message}\n");
        let _guard = match self.write_lock.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let _ = fs::create_dir_all(&self.logs_dir);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.today_log_file())
        {
            let _ = file.write_all(line.as_bytes());
        }
    }

    pub fn info(&self, message: &str) {
        self.log("INFO", message);
    }

    #[allow(dead_code)]
    pub fn warn(&self, message: &str) {
        self.log("WARN", message);
    }

    #[allow(dead_code)]
    pub fn error(&self, message: &str) {
        self.log("ERROR", message);
    }

    pub fn get_recent_logs(&self, limit: usize) -> Vec<String> {
        let mut files: Vec<PathBuf> = fs::read_dir(&self.logs_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok().map(|x| x.path()))
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .and_then(|s| s.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
            })
            .collect();
        files.sort();

        let mut lines = Vec::new();
        for path in files.into_iter().rev() {
            if let Ok(content) = fs::read_to_string(&path) {
                let mut file_lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
                file_lines.reverse();
                for line in file_lines {
                    lines.push(line);
                    if lines.len() >= limit {
                        lines.reverse();
                        return lines;
                    }
                }
            }
        }
        lines.reverse();
        lines
    }

    pub fn clean_old_logs(&self, days: Option<u64>) -> usize {
        let keep_days = days
            .filter(|&d| d > 0)
            .unwrap_or_else(|| self.retention_days.load(Ordering::SeqCst));
        let cutoff = SystemTime::now()
            .checked_sub(Duration::from_secs(keep_days.saturating_mul(86_400)))
            .unwrap_or(UNIX_EPOCH);

        let mut removed = 0usize;
        if let Ok(entries) = fs::read_dir(&self.logs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                if path
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_none_or(|e| !e.eq_ignore_ascii_case("log"))
                {
                    continue;
                }
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if modified < cutoff && fs::remove_file(&path).is_ok() {
                            removed += 1;
                        }
                    }
                }
            }
        }
        removed
    }
}
