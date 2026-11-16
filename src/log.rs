//! Lean rotating log file (3 files of about 1 MB each) in the data folder.
//! Contains only events and errors, never titles or note texts.
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const MAX_BYTES: u64 = 1024 * 1024;
const FILES: usize = 3;

static DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init(dir: &Path) {
    let _ = DIR.set(dir.to_path_buf());
}

pub fn file() -> Option<PathBuf> {
    DIR.get().map(|dir| dir.join("note-warden.log"))
}

fn rotated(base: &Path, index: usize) -> PathBuf {
    let mut name = base.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{index}"));
    base.with_file_name(name)
}

fn rotate(base: &Path) {
    let _ = fs::remove_file(rotated(base, FILES - 1));
    for index in (1..FILES - 1).rev() {
        let _ = fs::rename(rotated(base, index), rotated(base, index + 1));
    }
    let _ = fs::rename(base, rotated(base, 1));
}

fn write(level: &str, message: &str) {
    let Some(base) = file() else { return };
    if let Some(dir) = base.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if fs::metadata(&base).is_ok_and(|m| m.len() >= MAX_BYTES) {
        rotate(&base);
    }
    if let Ok(mut out) = OpenOptions::new().create(true).append(true).open(&base) {
        let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(out, "{stamp} {level} {message}");
    }
}

pub fn info(message: &str) {
    write("INFO ", message);
}

pub fn error(message: &str) {
    write("ERROR", message);
}

/// Panics end up in the log (without user content: only the location, no message text).
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let place = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unbekannt".into());
        error(&format!("Absturz in {place}"));
        previous(info);
    }));
}
