use crate::model::Workspace;
use directories::ProjectDirs;
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Loaded {
    pub workspace: Workspace,
    pub recovered: bool,
    /// Die Datei hatte ein älteres Schema und wurde migriert.
    pub migrated: bool,
    /// Zeitpunkt (Unix-ms) der Sicherung, aus der automatisch wiederhergestellt wurde.
    pub restored_from: Option<i64>,
}

fn fresh_workspace() -> Workspace {
    let mut workspace = Workspace::default();
    workspace.migrate();
    workspace
}

pub fn data_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("NOTE_WARDEN_DATA_DIR") {
        return PathBuf::from(dir).join("workspace.json");
    }
    ProjectDirs::from("", "Note Warden", "Note Warden")
        .map(|dirs| dirs.data_local_dir().join("workspace.json"))
        .unwrap_or_else(|| PathBuf::from("workspace.json"))
}

/// Liest Bytes, repariert IDs und hebt das Schema an. Zweiter Wert: vorherige Schemaversion, falls migriert.
fn decode(bytes: &[u8]) -> io::Result<(Workspace, Option<u32>)> {
    let mut workspace: Workspace = serde_json::from_slice(bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    workspace.repair_ids();
    let from = workspace.schema_version;
    let migrated = workspace.migrate();
    workspace.purge_trash(crate::model::today());
    Ok((workspace, migrated.then_some(from)))
}

/// Sichert die unveränderte Datei vor der ersten Migration (einmalig je Ausgangsversion):
/// Schema 3 -> `.bak-0.3.0`, ältere Dateien -> `.bak-0.2.0`.
fn backup_before_migration(path: &Path, bytes: &[u8], from: u32) -> io::Result<()> {
    let label = if from >= 7 {
        "0.9.2"
    } else if from >= 6 {
        "0.7.0"
    } else if from >= 5 {
        "0.6.0"
    } else if from >= 4 {
        "0.4.0"
    } else if from >= 3 {
        "0.3.0"
    } else {
        "0.2.0"
    };
    let backup = path.with_extension(format!("json.bak-{label}"));
    if !backup.exists() {
        write_sync(&backup, bytes)?;
    }
    write_backup_bytes(path, bytes, from, false)?;
    crate::log::info(&format!(
        "Migration von Schema {from} auf {}",
        crate::model::SCHEMA_VERSION
    ));
    Ok(())
}

/// Schreibt eine beliebige Datei atomar (temp-Datei, dann umbenennen).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    let temporary = path.with_file_name(name);
    write_sync(&temporary, bytes)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&temporary, path)
}

/// Liest ein JSON-Backup (für Vorschau/Import).
pub fn read_backup_file(path: &Path) -> io::Result<Workspace> {
    Ok(decode(&fs::read(path)?)?.0)
}

pub fn load(path: &Path) -> io::Result<Loaded> {
    match fs::read(path) {
        Ok(bytes) => match decode(&bytes) {
            Ok((mut workspace, from)) => {
                if let Some(modified) = fs::metadata(path)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                {
                    workspace.fill_created_at(modified.as_millis() as i64);
                }
                if let Some(from) = from {
                    backup_before_migration(path, &bytes, from)?;
                }
                Ok(Loaded {
                    workspace,
                    recovered: false,
                    migrated: from.is_some(),
                    restored_from: None,
                })
            }
            Err(error) => recover_automatically(path, error),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let backup = path.with_extension("json.bak");
            match fs::read(&backup) {
                Ok(bytes) => {
                    let (workspace, from) = decode(&bytes)?;
                    // An interrupted replacement may leave only the backup.
                    fs::copy(backup, path)?;
                    Ok(Loaded {
                        workspace,
                        recovered: true,
                        migrated: from.is_some(),
                        restored_from: None,
                    })
                }
                Err(backup_error) if backup_error.kind() == io::ErrorKind::NotFound => Ok(Loaded {
                    workspace: fresh_workspace(),
                    recovered: false,
                    migrated: false,
                    restored_from: None,
                }),
                Err(backup_error) => Err(backup_error),
            }
        }
        Err(error) => Err(error),
    }
}

/// Defekte Datendatei: neueste gültige Sicherung laden, die defekte Datei als
/// `*.corrupt-<Zeitstempel>` ablegen (nie löschen). Ohne gültige Sicherung bleibt der Fehler.
fn recover_automatically(path: &Path, original: io::Error) -> io::Result<Loaded> {
    crate::log::error("Datendatei nicht lesbar, suche Sicherung");
    let mut candidates: Vec<(PathBuf, i64)> = Vec::new();
    let previous = path.with_extension("json.bak");
    if let Ok(modified) = fs::metadata(&previous).and_then(|m| m.modified()) {
        candidates.push((previous, millis(modified)));
    }
    candidates.extend(list_backups(path).into_iter().map(|b| (b.path, b.at)));
    for (candidate, at) in candidates {
        let Ok(bytes) = fs::read(&candidate) else {
            continue;
        };
        let Ok((workspace, from)) = decode(&bytes) else {
            continue;
        };
        let invalid = path.with_extension(format!(
            "json.corrupt-{}",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        ));
        fs::rename(path, &invalid)?;
        write_atomic(path, &bytes)?;
        crate::log::info("Aus Sicherung wiederhergestellt");
        return Ok(Loaded {
            workspace,
            recovered: true,
            migrated: from.is_some(),
            restored_from: Some(at),
        });
    }
    Err(original)
}

fn millis(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------- Backups (Ordner `backups/` neben der Datendatei) ----------

#[derive(Clone)]
pub struct BackupInfo {
    pub path: PathBuf,
    /// Unix-Millisekunden, aus dem Dateinamen.
    pub at: i64,
    /// Manuell angelegt: wird nie automatisch gelöscht.
    pub pinned: bool,
    pub size: u64,
}

pub fn backups_dir(path: &Path) -> PathBuf {
    path.with_file_name("backups")
}

pub fn data_dir(path: &Path) -> PathBuf {
    path.parent().map(Path::to_path_buf).unwrap_or_default()
}

fn parse_backup_name(name: &str) -> Option<(i64, bool)> {
    use chrono::{Local, NaiveDateTime, TimeZone};
    let rest = name.strip_prefix("workspace-")?.strip_suffix(".json")?;
    let (rest, pinned) = rest
        .strip_suffix("-manual")
        .map_or((rest, false), |r| (r, true));
    let stamp = rest.get(..15)?;
    let naive = NaiveDateTime::parse_from_str(stamp, "%Y%m%d-%H%M%S").ok()?;
    let at = Local
        .from_local_datetime(&naive)
        .earliest()?
        .timestamp_millis();
    Some((at, pinned))
}

/// Alle Backups, neueste zuerst.
pub fn list_backups(path: &Path) -> Vec<BackupInfo> {
    let Ok(entries) = fs::read_dir(backups_dir(path)) else {
        return Vec::new();
    };
    let mut list: Vec<BackupInfo> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let (at, pinned) = parse_backup_name(&name)?;
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            Some(BackupInfo {
                path: entry.path(),
                at,
                pinned,
                size,
            })
        })
        .collect();
    list.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| b.path.cmp(&a.path)));
    list
}

fn write_backup_bytes(path: &Path, bytes: &[u8], schema: u32, pinned: bool) -> io::Result<PathBuf> {
    let dir = backups_dir(path);
    fs::create_dir_all(&dir)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let suffix = if pinned { "-manual" } else { "" };
    let target = dir.join(format!("workspace-{stamp}-v{schema}{suffix}.json"));
    write_atomic(&target, bytes)?;
    Ok(target)
}

/// Legt ein Backup im Format der Datendatei an. Automatische Backups räumen danach auf.
pub fn create_backup(path: &Path, workspace: &Workspace, pinned: bool) -> io::Result<PathBuf> {
    let bytes = serde_json::to_vec_pretty(workspace).map_err(io::Error::other)?;
    let target = write_backup_bytes(path, &bytes, workspace.schema_version, pinned)?;
    if !pinned {
        prune_backups(path);
    }
    crate::log::info(if pinned {
        "Manuelles Backup angelegt"
    } else {
        "Backup angelegt"
    });
    Ok(target)
}

/// Welche Backups bleiben? `entries` = (Zeitpunkt in Unix-ms, gepinnt); Ergebnis gleich lang.
/// Behalten: gepinnte, die 3 neuesten insgesamt, die 10 neuesten automatischen, je das neueste
/// pro Tag der letzten 7 Tage und je das neueste pro Woche der letzten 4 Wochen.
pub fn retention(entries: &[(i64, bool)], now: i64) -> Vec<bool> {
    use chrono::{Datelike, Local, TimeZone};
    const DAY: i64 = 86_400_000;
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by(|a, b| entries[*b].0.cmp(&entries[*a].0));
    let mut keep = vec![false; entries.len()];
    let mut days = std::collections::BTreeSet::new();
    let mut weeks = std::collections::BTreeSet::new();
    let mut automatic = 0;
    for (rank, &index) in order.iter().enumerate() {
        let (at, pinned) = entries[index];
        if pinned || rank < 3 {
            keep[index] = true;
        }
        if pinned {
            continue;
        }
        automatic += 1;
        if automatic <= 10 {
            keep[index] = true;
        }
        let Some(local) = Local.timestamp_millis_opt(at).single() else {
            continue;
        };
        let age = now - at;
        if age < 7 * DAY && days.insert(local.date_naive()) {
            keep[index] = true;
        }
        let week = local.iso_week();
        if age < 28 * DAY && weeks.insert((week.year(), week.week())) {
            keep[index] = true;
        }
    }
    keep
}

pub fn prune_backups(path: &Path) {
    let list = list_backups(path);
    let entries: Vec<(i64, bool)> = list.iter().map(|b| (b.at, b.pinned)).collect();
    let now = chrono::Local::now().timestamp_millis();
    for (backup, keep) in list.iter().zip(retention(&entries, now)) {
        if !keep {
            let _ = fs::remove_file(&backup.path);
        }
    }
}

// ---------- Versionsverlauf der Notiztexte (`history/<update-id>.json`) ----------

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub at: i64,
    pub text: String,
}

pub const MAX_SNAPSHOTS: usize = 30;
pub const SNAPSHOT_GAP_MS: i64 = 5 * 60 * 1000;

fn history_file(path: &Path, update_id: u64) -> PathBuf {
    path.with_file_name("history")
        .join(format!("{update_id}.json"))
}

/// Fehlende oder defekte Dateien ergeben einen leeren Verlauf.
pub fn read_history(path: &Path, update_id: u64) -> Vec<Snapshot> {
    fs::read(history_file(path, update_id))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Snapshotted {
    Saved,
    /// Text gleicht dem letzten Snapshot (oder ist leer): nichts zu tun.
    Unchanged,
    /// Letzter Snapshot ist jünger als 5 Minuten: später erneut versuchen.
    TooSoon,
    Failed,
}

/// Speichert einen Snapshot, wenn sich der Text zum letzten unterscheidet. `force` ignoriert
/// den Mindestabstand von 5 Minuten (Verlassen des Updates, Beenden, Wiederherstellen).
pub fn add_snapshot(path: &Path, update_id: u64, text: &str, now: i64, force: bool) -> Snapshotted {
    let mut list = read_history(path, update_id);
    if list.last().is_some_and(|last| last.text == text)
        || (list.is_empty() && text.trim().is_empty())
    {
        return Snapshotted::Unchanged;
    }
    if !force
        && list
            .last()
            .is_some_and(|last| now - last.at < SNAPSHOT_GAP_MS)
    {
        return Snapshotted::TooSoon;
    }
    list.push(Snapshot {
        at: now,
        text: text.to_owned(),
    });
    while list.len() > MAX_SNAPSHOTS {
        list.remove(1);
    }
    let Ok(bytes) = serde_json::to_vec(&list) else {
        return Snapshotted::Failed;
    };
    let file = history_file(path, update_id);
    if let Some(dir) = file.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if write_atomic(&file, &bytes).is_ok() {
        Snapshotted::Saved
    } else {
        Snapshotted::Failed
    }
}

/// Entfernt Verläufe von Updates, die es nicht mehr gibt (endgültig gelöscht).
pub fn prune_history(path: &Path, existing: &std::collections::BTreeSet<u64>) {
    let Ok(entries) = fs::read_dir(path.with_file_name("history")) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let id = name
            .strip_suffix(".json")
            .and_then(|n| n.parse::<u64>().ok());
        if id.is_some_and(|id| !existing.contains(&id)) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Minimale Kopie von Theme/Akzent/Fenster, die vor dem Laden des Arbeitsbereichs
/// gelesen wird (kein Aufblitzen im falschen Theme, Fenstergröße vor dem Öffnen bekannt).
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Prefs {
    #[serde(default)]
    pub theme: crate::model::ThemeChoice,
    #[serde(default)]
    pub accent: crate::model::Accent,
    #[serde(default)]
    pub window_size: Option<(f32, f32)>,
}

impl From<&crate::model::Settings> for Prefs {
    fn from(s: &crate::model::Settings) -> Self {
        Self {
            theme: s.theme,
            accent: s.accent,
            window_size: s.remember_window.then_some(s.window_size).flatten(),
        }
    }
}

fn prefs_path(workspace: &Path) -> PathBuf {
    workspace.with_file_name("prefs.json")
}

pub fn read_prefs(workspace: &Path) -> Prefs {
    fs::read(prefs_path(workspace))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn write_prefs(workspace: &Path, prefs: &Prefs) {
    if let Ok(bytes) = serde_json::to_vec(prefs) {
        let _ = write_atomic(&prefs_path(workspace), &bytes);
    }
}

fn write_sync(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Called on the GPUI thread. Calls complete in order, so an older snapshot
/// cannot finish after a newer one. A failure leaves the in-memory workspace
/// untouched and can be retried from the same state.
pub fn save(path: &Path, workspace: &Workspace) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(workspace).map_err(io::Error::other)?;
    let temporary = path.with_extension("json.tmp");
    write_sync(&temporary, &bytes)?;
    if path.exists() {
        let backup = path.with_extension("json.bak");
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        fs::rename(path, &backup)?;
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::rename(backup, path);
            return Err(error);
        }
    } else {
        fs::rename(&temporary, path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("note-warden-{label}-{}", std::process::id()))
            .join("workspace.json")
    }

    #[test]
    fn v04_backup_and_defaults() {
        let path = temp_path("migration04");
        let source = br#"{"schema_version":4,"updates":[{"id":1,"title":"Alt"}],"next_id":2}"#;
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source).unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.migrated);
        assert_eq!(
            loaded.workspace.schema_version,
            crate::model::SCHEMA_VERSION
        );
        assert_eq!(loaded.workspace.updates[0].priority, 0);
        assert!(
            loaded.workspace.updates[0].version.is_empty()
                && loaded.workspace.updates[0].activity.is_empty()
        );
        assert_eq!(loaded.workspace.updates[0].project_id, None);
        assert!(loaded.workspace.projects.is_empty());
        assert!(loaded.workspace.standalone_tasks.is_empty());
        assert_eq!(
            fs::read(path.with_extension("json.bak-0.4.0")).unwrap(),
            source
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn v05_gets_bak_0_6_0_and_current_schema() {
        let path = temp_path("migration05");
        let source = br#"{"schema_version":5,"updates":[{"id":1,"title":"Alt","tasks":[{"id":2,"title":"A"}]}],"next_id":3}"#;
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source).unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.migrated);
        assert_eq!(
            loaded.workspace.schema_version,
            crate::model::SCHEMA_VERSION
        );
        assert_eq!(loaded.workspace.updates[0].tasks[0].description, "");
        assert_eq!(
            fs::read(path.with_extension("json.bak-0.6.0")).unwrap(),
            source
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn v01_workspace_roundtrips_without_losing_fields() {
        let path = temp_path("migration");
        let source = r#"{"updates":[{"id":7,"title":"Update 1.4","notes":"Erste Zeile\nÄnderung: ü ✓","status":"in_arbeit","planned_for":"2026-12-02","tasks":[{"id":9,"title":"Feature A","done":true,"planned_for":"2026-12-01"}]}],"next_id":10}"#;
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source.as_bytes()).unwrap();
        let first = load(&path).unwrap();
        assert!(first.migrated);
        let backup = path.with_extension("json.bak-0.2.0");
        assert_eq!(fs::read(&backup).unwrap(), source.as_bytes());
        let mut loaded = first.workspace;
        assert_eq!(loaded.schema_version, crate::model::SCHEMA_VERSION);
        assert!(!loaded.updates[0].archived && loaded.updates[0].deleted_on.is_none());
        assert!(!loaded.updates[0].tasks[0].sub);
        assert_eq!(loaded.updates[0].notes, "Erste Zeile\nÄnderung: ü ✓");
        assert_eq!(
            loaded.updates[0].tasks[0].planned_for,
            NaiveDate::from_ymd_opt(2026, 12, 1)
        );
        assert_eq!(loaded.new_update(None), 10);
        save(&path, &loaded).unwrap();
        assert_eq!(load(&path).unwrap().workspace.updates[0].id, 7);
        assert!(!load(&path).unwrap().migrated);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn v03_workspace_gets_defaults_and_bak_0_3_0() {
        let path = temp_path("migration03");
        let source = r#"{"schema_version":3,"updates":[],"next_id":1,"settings":{"theme":"light","week_start":"sunday","list_width":400.0,"templates":[]}}"#;
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source.as_bytes()).unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.migrated);
        assert_eq!(
            fs::read(path.with_extension("json.bak-0.3.0")).unwrap(),
            source.as_bytes()
        );
        let s = &loaded.workspace.settings;
        assert!(s.theme == crate::model::ThemeChoice::Light);
        assert!(s.week_start == crate::model::WeekStart::Sunday);
        assert_eq!(s.max_pills, 3);
        assert_eq!(s.trash_days, 30);
        assert!(s.undo_instead_of_dialog && s.word_wrap);
        assert_eq!(s.autosave_ms, 400);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn backup_restores_missing_or_corrupt_primary_automatically() {
        let path = temp_path("recovery");
        let mut workspace = Workspace::default();
        workspace.new_update(None);
        save(&path, &workspace).unwrap();
        workspace.new_update(None);
        save(&path, &workspace).unwrap();
        fs::remove_file(&path).unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.recovered);
        assert_eq!(loaded.workspace.updates.len(), 1);
        fs::write(&path, b"invalid json").unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.restored_from.is_some());
        assert_eq!(loaded.workspace.updates.len(), 1);
        let kept = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().contains(".corrupt-"));
        assert!(kept);
        assert_eq!(load(&path).unwrap().workspace.updates.len(), 1);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn later_synchronous_save_wins() {
        let path = temp_path("ordering");
        let mut workspace = Workspace::default();
        let id = workspace.new_update(None);
        save(&path, &workspace).unwrap();
        workspace.update_mut(id).unwrap().title = "letzter Stand".into();
        save(&path, &workspace).unwrap();
        assert_eq!(
            load(&path).unwrap().workspace.update(id).unwrap().title,
            "letzter Stand"
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn failed_save_can_retry_without_losing_the_workspace() {
        let path = temp_path("retry");
        let mut workspace = Workspace::default();
        let id = workspace.new_update(None);
        workspace.update_mut(id).unwrap().notes = "Zeile 1\nÄnderung ✓".into();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let temporary = path.with_extension("json.tmp");
        fs::create_dir(&temporary).unwrap();
        assert!(save(&path, &workspace).is_err());
        assert_eq!(workspace.update(id).unwrap().notes, "Zeile 1\nÄnderung ✓");
        fs::remove_dir(&temporary).unwrap();
        save(&path, &workspace).unwrap();
        assert_eq!(
            load(&path).unwrap().workspace.update(id).unwrap().notes,
            "Zeile 1\nÄnderung ✓"
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn retention_keeps_newest_recent_days_weeks_and_pinned() {
        use chrono::{Local, TimeZone};
        const MIN: i64 = 60_000;
        const DAY: i64 = 86_400_000;
        let now = Local
            .with_ymd_and_hms(2026, 11, 15, 12, 0, 0)
            .unwrap()
            .timestamp_millis();
        let mut entries: Vec<(i64, bool)> = (0..12).map(|i| (now - i * 10 * MIN, false)).collect();
        entries.push((now - 60 * DAY, true));
        entries.push((now - 20 * DAY, false));
        entries.push((now - 20 * DAY - 60 * MIN, false));
        entries.push((now - 40 * DAY, false));
        let keep = retention(&entries, now);
        assert_eq!(
            &keep[..12],
            &[
                true, true, true, true, true, true, true, true, true, true, false, false
            ]
        );
        assert_eq!(&keep[12..], &[true, true, false, false]);
    }
}
