use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visited_at: DateTime<Utc>,
}

pub struct HistoryStore {
    path: PathBuf,
    entries: Vec<HistoryEntry>,
    max_entries: usize,
}

impl HistoryStore {
    pub fn load(path: PathBuf) -> Self {
        let entries = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Self {
            path,
            entries,
            max_entries: 500,
        }
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn record(&mut self, url: String, title: String) {
        if url.is_empty() || url == "about:blank" {
            return;
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

        self.persist();
    }

    fn persist(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.entries) {
            let _ = fs::write(&self.path, json);
        }
    }
}
