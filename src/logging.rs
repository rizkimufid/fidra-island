//! Lightweight rotating structured file logger.
//!
//! Writes timestamped log records under `%LOCALAPPDATA%\FidraIsland\logs\fidra-island.log`
//! so production issues can be diagnosed even when running with `windows_subsystem = "windows"`.
//! Automatically rotates the active log file to `fidra-island.log.1` when it exceeds `2 MiB`.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

static LOGGER: OnceLock<Mutex<FileLogger>> = OnceLock::new();

struct FileLogger {
    path: PathBuf,
}

impl FileLogger {
    fn write_line(&self, level: &str, target: &str, message: &str) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        rotate_if_needed(&self.path, MAX_LOG_BYTES);

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let ts = format_unix_timestamp(SystemTime::now());
            let _ = writeln!(file, "{ts} [{level}] [{target}] {message}");
        }
    }
}

/// Returns the log directory (`%LOCALAPPDATA%\FidraIsland\logs` on Windows).
pub fn log_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("FidraIsland").join("logs")
}

/// Initializes the global rotating file logger. Safe to call once at startup.
pub fn init() {
    let dir = log_dir();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("fidra-island.log");
    let _ = LOGGER.set(Mutex::new(FileLogger { path }));
}

/// Writes a structured log record to the active log file.
pub fn log(level: &str, target: &str, message: &str) {
    if let Some(logger) = LOGGER.get() {
        if let Ok(guard) = logger.lock() {
            guard.write_line(level, target, message);
        }
    }
}

/// Logs an informational message.
pub fn info(target: &str, message: impl AsRef<str>) {
    log("INFO", target, message.as_ref());
}

/// Logs a warning message.
pub fn warn(target: &str, message: impl AsRef<str>) {
    log("WARN", target, message.as_ref());
}

/// Logs an error message.
pub fn error(target: &str, message: impl AsRef<str>) {
    log("ERROR", target, message.as_ref());
}

fn rotate_if_needed(path: &Path, max_bytes: u64) {
    if let Ok(meta) = fs::metadata(path) {
        if meta.len() >= max_bytes {
            let backup = path.with_extension("log.1");
            let _ = fs::remove_file(&backup);
            let _ = fs::rename(path, &backup);
        }
    }
}

fn format_unix_timestamp(now: SystemTime) -> String {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Civil date conversion from Unix epoch seconds (UTC).
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's civil-from-days algorithm (UTC, zero external chrono crate).
fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_utc_epoch_timestamps() {
        assert_eq!(format_unix_timestamp(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        let y2k = UNIX_EPOCH + std::time::Duration::from_secs(946_684_800);
        assert_eq!(format_unix_timestamp(y2k), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn rotates_log_when_threshold_exceeded() {
        let dir = std::env::temp_dir().join(format!("fidra_log_rot_{}", std::process::id()));
        let path = dir.join("fidra-island.log");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "0123456789").unwrap();

        rotate_if_needed(&path, 5);
        assert!(!path.exists());
        assert!(path.with_extension("log.1").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
