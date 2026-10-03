use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visited_at: DateTime<Utc>,
}

/// Browsing history kept in memory and written to disk in batches (X-17):
/// `record` only marks the store dirty, and the owner calls `flush` shortly
/// afterwards and on shutdown. Writes are atomic and private to the user, and a
/// file that cannot be parsed is set aside instead of being overwritten.
pub struct HistoryStore {
    path: PathBuf,
    entries: Vec<HistoryEntry>,
    max_entries: usize,
    dirty: bool,
}

impl HistoryStore {
    pub fn load(path: PathBuf) -> Self {
        let entries = match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str(&raw) {
                Ok(entries) => entries,
                Err(err) => {
                    set_aside_corrupt_file(&path, &err.to_string());
                    Vec::new()
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(err) => {
                set_aside_corrupt_file(&path, &err.to_string());
                Vec::new()
            }
        };

        Self {
            path,
            entries,
            max_entries: 500,
            dirty: false,
        }
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Records a visit. Returns true when the store just became dirty, i.e.
    /// when the caller should schedule a `flush`.
    pub fn record(&mut self, url: String, title: String) -> bool {
        if url.is_empty() || url == "about:blank" {
            return false;
        }

        self.entries.retain(|e| e.url != url);
        self.entries.insert(
            0,
            HistoryEntry {
                url,
                title,
                visited_at: Utc::now(),
            },
        );

        if self.entries.len() > self.max_entries {
            self.entries.truncate(self.max_entries);
        }

        let schedule = !self.dirty;
        self.dirty = true;
        schedule
    }

    /// Writes pending changes to disk, if any.
    pub fn flush(&mut self) {
        if !self.dirty {
            return;
        }
        match serde_json::to_string_pretty(&self.entries) {
            Ok(json) => match write_private_atomically(&self.path, json.as_bytes()) {
                Ok(()) => self.dirty = false,
                Err(err) => eprintln!("Could not save history to {}: {err}", self.path.display()),
            },
            Err(err) => eprintln!("Could not serialise history: {err}"),
        }
    }
}

/// Keeps an unreadable history file for the user to inspect instead of
/// silently replacing it with an empty one on the next save.
fn set_aside_corrupt_file(path: &Path, reason: &str) {
    let mut backup = path.as_os_str().to_owned();
    backup.push(format!(".corrupt-{}", Utc::now().format("%Y%m%dT%H%M%S")));
    let backup = PathBuf::from(backup);
    match fs::rename(path, &backup) {
        Ok(()) => eprintln!(
            "History file {} could not be read ({reason}); moved it to {} and started empty",
            path.display(),
            backup.display()
        ),
        Err(err) => eprintln!(
            "History file {} could not be read ({reason}) or moved aside ({err})",
            path.display()
        ),
    }
}

/// Writes `contents` to a 0600 temporary file next to `path`, syncs it and
/// renames it over `path`, so a crash never leaves a truncated file.
#[cfg(unix)]
fn write_private_atomically(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;

    if let Some(parent) = path.parent() {
        crate::profile::ensure_private_dir(parent)?;
    }
    let mut temp = path.as_os_str().to_owned();
    temp.push(format!(".tmp-{}", std::process::id()));
    let temp = PathBuf::from(temp);
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temp)?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(all(test, unix))]
mod tests {
    use super::HistoryStore;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nodaysidle-history-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn history_is_saved_atomically_with_private_permissions() {
        let dir = test_dir("atomic");
        let path = dir.join("profile/history.json");
        let mut store = HistoryStore::load(path.clone());
        assert!(store.record("https://example.com/".into(), "Example".into()));
        // Further visits before the flush do not schedule another save.
        assert!(!store.record("https://example.org/".into(), "Org".into()));
        assert!(!path.exists(), "record must not write synchronously");

        store.flush();
        let file_mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(file_mode & 0o777, 0o600);
        let dir_mode = std::fs::metadata(path.parent().unwrap()).unwrap().permissions().mode();
        assert_eq!(dir_mode & 0o777, 0o700);
        let leftovers = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter(|entry| entry.as_ref().unwrap().file_name() != "history.json")
            .count();
        assert_eq!(leftovers, 0, "no temporary files are left behind");

        let reloaded = HistoryStore::load(path);
        let urls: Vec<_> = reloaded.entries().iter().map(|e| e.url.as_str()).collect();
        assert_eq!(urls, ["https://example.org/", "https://example.com/"]);
        assert!(store.record("https://example.net/".into(), "Net".into()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_corrupt_history_file_is_set_aside_not_wiped() {
        let dir = test_dir("corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.json");
        std::fs::write(&path, b"{ not json").unwrap();

        let mut store = HistoryStore::load(path.clone());
        assert!(store.entries().is_empty());
        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.starts_with("history.json.corrupt-"))
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(std::fs::read(dir.join(&backups[0])).unwrap(), b"{ not json");

        store.record("https://example.com/".into(), "Example".into());
        store.flush();
        assert_eq!(HistoryStore::load(path).entries().len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
