use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How long after the first unsaved visit the batch is written (X-17).
pub const SAVE_DELAY: Duration = Duration::from_secs(2);
/// First retry delay after a failed save; doubles up to `MAX_RETRY_DELAY`.
const FIRST_RETRY_DELAY: Duration = Duration::from_secs(5);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visited_at: DateTime<Utc>,
}

/// Browsing history kept in memory and written to disk in batches (X-17):
/// `record` marks the store dirty and asks the owner to arm one save timer
/// (`SAVE_DELAY` after the first unsaved visit); the timer calls
/// `begin_background_save`, writes the snapshot off the main thread and hands
/// the result to `finish_background_save`, which asks for a retry with backoff
/// if the write failed (V-3). The owner also calls `flush` on shutdown. Writes are atomic and
/// private to the user, and a file that cannot be parsed is set aside instead
/// of being overwritten.
pub struct HistoryStore {
    path: PathBuf,
    entries: Vec<HistoryEntry>,
    max_entries: usize,
    dirty: bool,
    /// A save timer is armed and will call `flush_from_timer`.
    save_pending: bool,
    retry_delay: Duration,
    /// Bumped by every change, so a save knows whether it wrote the latest state.
    generation: u64,
    /// Serialises writes and holds the generation last written to disk, so a
    /// slow background save can never replace a newer file (e.g. a clear).
    disk: Arc<Mutex<u64>>,
}

/// A snapshot of the history to write on a worker thread.
pub struct BackgroundSave {
    path: PathBuf,
    json: String,
    generation: u64,
    disk: Arc<Mutex<u64>>,
}

impl BackgroundSave {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Writes the snapshot unless newer history is already on disk.
    pub fn write(self) -> bool {
        let mut on_disk = self
            .disk
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if *on_disk >= self.generation {
            return true;
        }
        match write_private_atomically(&self.path, self.json.as_bytes()) {
            Ok(()) => {
                *on_disk = self.generation;
                true
            }
            Err(err) => {
                eprintln!("Could not save history to {}: {err}", self.path.display());
                false
            }
        }
    }
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
            save_pending: false,
            retry_delay: FIRST_RETRY_DELAY,
            generation: 0,
            disk: Arc::new(Mutex::new(0)),
        }
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Records a visit. Returns the delay after which the caller must call
    /// `flush_from_timer` when no save timer is armed yet, otherwise `None`.
    pub fn record(&mut self, url: String, title: String) -> Option<Duration> {
        if url.is_empty() || url == "about:blank" {
            return None;
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

        self.dirty = true;
        self.generation += 1;
        if self.save_pending {
            return None;
        }
        self.save_pending = true;
        Some(SAVE_DELAY)
    }

    /// Called by the save timer on the main thread: snapshots pending changes
    /// for `BackgroundSave::write`, or returns None when there is nothing to save.
    pub fn begin_background_save(&mut self) -> Option<BackgroundSave> {
        if !self.dirty {
            self.save_pending = false;
            return None;
        }
        let json = match serde_json::to_string_pretty(&self.entries) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("Could not serialise history: {err}");
                self.save_pending = false;
                return None;
            }
        };
        Some(BackgroundSave {
            path: self.path.clone(),
            json,
            generation: self.generation,
            disk: self.disk.clone(),
        })
    }

    /// Applies a background save's result. Returns the delay after which the
    /// caller must save again: a retry with backoff (doubling up to a minute)
    /// after a failure, or `SAVE_DELAY` when visits arrived during the write.
    pub fn finish_background_save(&mut self, generation: u64, ok: bool) -> Option<Duration> {
        self.save_pending = false;
        if ok {
            self.retry_delay = FIRST_RETRY_DELAY;
            if self.generation == generation {
                self.dirty = false;
                return None;
            }
            self.save_pending = true;
            return Some(SAVE_DELAY);
        }
        let delay = self.retry_delay;
        self.retry_delay = (self.retry_delay * 2).min(MAX_RETRY_DELAY);
        self.save_pending = true;
        Some(delay)
    }

    /// The save-timer path run synchronously (begin, write, finish), for tests.
    #[cfg(test)]
    pub fn flush_from_timer(&mut self) -> Option<Duration> {
        let save = self.begin_background_save()?;
        let generation = save.generation();
        let ok = save.write();
        self.finish_background_save(generation, ok)
    }

    /// Removes all entries from memory and writes the empty list to disk.
    /// Existing save callbacks flush current entries, so they cannot restore
    /// cleared visits; failed writes leave the store dirty for a later retry.
    pub fn clear(&mut self) -> bool {
        self.entries.clear();
        self.dirty = true;
        self.generation += 1;
        self.save_pending = false;
        self.retry_delay = FIRST_RETRY_DELAY;
        self.remove_aside_files();
        self.flush()
    }

    /// Set-aside corrupt copies and leftover temp files can still hold old
    /// visits, so clearing history removes them too.
    fn remove_aside_files(&self) {
        let (Some(dir), Some(name)) = (self.path.parent(), self.path.file_name()) else {
            return;
        };
        let name = name.to_string_lossy();
        let (corrupt, temp) = (format!("{name}.corrupt-"), format!("{name}.tmp-"));
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let file = entry.file_name();
            let file = file.to_string_lossy();
            if file.starts_with(&corrupt) || file.starts_with(&temp) {
                if let Err(err) = fs::remove_file(entry.path()) {
                    eprintln!("Could not remove {}: {err}", entry.path().display());
                }
            }
        }
    }

    /// Writes pending changes to disk; failed writes leave the store dirty.
    pub fn flush(&mut self) -> bool {
        if !self.dirty {
            return true;
        }
        let mut on_disk = self
            .disk
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let result = serde_json::to_string_pretty(&self.entries)
            .map_err(std::io::Error::from)
            .and_then(|json| write_private_atomically(&self.path, json.as_bytes()));
        match result {
            Ok(()) => {
                *on_disk = self.generation;
                self.dirty = false;
                true
            }
            Err(err) => {
                eprintln!("Could not save history to {}: {err}", self.path.display());
                false
            }
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
        crate::profile::ensure_private_dir(parent)
            .map_err(|err| std::io::Error::other(err.to_string()))?;
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
    use super::{HistoryStore, SAVE_DELAY};
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::time::Duration;

    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("nodaysidle-history-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn history_is_saved_atomically_with_private_permissions() {
        let dir = test_dir("atomic");
        let path = dir.join("profile/history.json");
        let mut store = HistoryStore::load(path.clone());
        assert_eq!(
            store.record("https://example.com/".into(), "Example".into()),
            Some(SAVE_DELAY)
        );
        // Further visits before the flush do not schedule another save.
        assert_eq!(
            store.record("https://example.org/".into(), "Org".into()),
            None
        );
        assert!(!path.exists(), "record must not write synchronously");

        assert_eq!(store.flush_from_timer(), None);
        let file_mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(file_mode & 0o777, 0o600);
        let dir_mode = std::fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(dir_mode & 0o777, 0o700);
        let leftovers = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter(|entry| entry.as_ref().unwrap().file_name() != "history.json")
            .count();
        assert_eq!(leftovers, 0, "no temporary files are left behind");

        let reloaded = HistoryStore::load(path);
        let urls: Vec<_> = reloaded.entries().iter().map(|e| e.url.as_str()).collect();
        assert_eq!(urls, ["https://example.org/", "https://example.com/"]);
        assert_eq!(
            store.record("https://example.net/".into(), "Net".into()),
            Some(SAVE_DELAY)
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failed_save_is_retried_with_backoff_until_it_succeeds() {
        let dir = test_dir("retry");
        std::fs::create_dir_all(&dir).unwrap();
        // A regular file where the profile directory should be makes the
        // write fail regardless of the user running the test.
        let blocker = dir.join("profile");
        std::fs::write(&blocker, b"").unwrap();
        let path = blocker.join("history.json");
        let mut store = HistoryStore::load(path.clone());

        assert_eq!(
            store.record("https://example.com/".into(), "Example".into()),
            Some(SAVE_DELAY)
        );
        assert_eq!(store.flush_from_timer(), Some(Duration::from_secs(5)));
        // The retry timer is pending, so new visits do not arm another one.
        assert_eq!(
            store.record("https://example.org/".into(), "Org".into()),
            None
        );
        assert_eq!(store.flush_from_timer(), Some(Duration::from_secs(10)));

        std::fs::remove_file(&blocker).unwrap();
        assert_eq!(store.flush_from_timer(), None);
        let reloaded = HistoryStore::load(path);
        let urls: Vec<_> = reloaded.entries().iter().map(|e| e.url.as_str()).collect();
        assert_eq!(urls, ["https://example.org/", "https://example.com/"]);
        // Back to normal batching, with the backoff reset.
        assert_eq!(
            store.record("https://example.net/".into(), "Net".into()),
            Some(SAVE_DELAY)
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cleared_history_stays_empty_after_reload() {
        let dir = test_dir("clear");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.json");
        let mut store = HistoryStore::load(path.clone());
        store.record("https://example.com/".into(), "Example".into());
        assert!(store.clear());
        assert!(store.entries().is_empty());
        let reloaded = HistoryStore::load(path);
        assert!(reloaded.entries().is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clear_then_pending_timer_does_not_restore_history() {
        for fail_clear in [false, true] {
            let dir = test_dir(if fail_clear {
                "clear-timer-fail"
            } else {
                "clear-timer"
            });
            let path = dir.join("history.json");
            let mut store = HistoryStore::load(path.clone());
            assert_eq!(
                store.record("https://old.example/".into(), "Old".into()),
                Some(SAVE_DELAY)
            );
            assert!(store.flush());
            if fail_clear {
                std::fs::create_dir(
                    path.with_extension(format!("json.tmp-{}", std::process::id())),
                )
                .unwrap();
            }
            assert_eq!(store.clear(), !fail_clear);
            if fail_clear {
                assert_eq!(HistoryStore::load(path.clone()).entries().len(), 1);
                assert_eq!(store.flush_from_timer(), Some(Duration::from_secs(5)));
                std::fs::remove_dir(
                    path.with_extension(format!("json.tmp-{}", std::process::id())),
                )
                .unwrap();
            }
            assert_eq!(store.flush_from_timer(), None);
            assert!(HistoryStore::load(path).entries().is_empty());
            std::fs::remove_dir_all(dir).unwrap();
        }
    }

    #[test]
    fn clear_then_record_then_old_and_new_timers_save_only_new_history() {
        for fail_clear in [false, true] {
            let dir = test_dir(if fail_clear {
                "clear-record-fail"
            } else {
                "clear-record"
            });
            let path = dir.join("history.json");
            let mut store = HistoryStore::load(path.clone());
            assert_eq!(
                store.record("https://old.example/".into(), "Old".into()),
                Some(SAVE_DELAY)
            );
            assert!(store.flush());
            let blocker = path.with_extension(format!("json.tmp-{}", std::process::id()));
            if fail_clear {
                std::fs::create_dir(&blocker).unwrap();
            }
            assert_eq!(store.clear(), !fail_clear);
            assert_eq!(
                store.record("https://new.example/".into(), "New".into()),
                Some(SAVE_DELAY)
            );
            if fail_clear {
                assert_eq!(store.flush_from_timer(), Some(Duration::from_secs(5)));
                std::fs::remove_dir(blocker).unwrap();
            }
            // The old callback flushes current entries; the new callback is harmless.
            assert_eq!(store.flush_from_timer(), None);
            assert_eq!(store.flush_from_timer(), None);
            let reloaded = HistoryStore::load(path);
            assert_eq!(reloaded.entries().len(), 1);
            assert_eq!(reloaded.entries()[0].url, "https://new.example/");
            std::fs::remove_dir_all(dir).unwrap();
        }
    }

    #[test]
    fn a_background_save_never_overwrites_newer_history() {
        let dir = test_dir("background");
        let path = dir.join("history.json");
        let mut store = HistoryStore::load(path.clone());
        store.record("https://a.example/".into(), "A".into());

        // A visit while the write runs keeps the store dirty and asks for another save.
        let save = store.begin_background_save().unwrap();
        let generation = save.generation();
        store.record("https://b.example/".into(), "B".into());
        assert!(save.write());
        assert_eq!(
            store.finish_background_save(generation, true),
            Some(SAVE_DELAY)
        );
        assert_eq!(store.flush_from_timer(), None);
        assert_eq!(HistoryStore::load(path.clone()).entries().len(), 2);

        // A clear that lands before an older snapshot is written wins on disk.
        store.record("https://c.example/".into(), "C".into());
        let stale = store.begin_background_save().unwrap();
        assert!(store.clear());
        assert!(stale.write());
        assert!(HistoryStore::load(path).entries().is_empty());
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
        assert_eq!(HistoryStore::load(path.clone()).entries().len(), 1);

        // Clearing history must not leave the old visits in the backup.
        std::fs::write(dir.join("history.json.tmp-1"), b"[]").unwrap();
        assert!(store.clear());
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name != "history.json")
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
        assert!(HistoryStore::load(path).entries().is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
