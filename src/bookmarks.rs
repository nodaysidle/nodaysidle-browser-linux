//! Bookmarks management: persist user favorites in ~/.local/share/nodaysidle-browser/bookmarks.json
//! and synchronize with the active Omarchy theme.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
}

fn bookmarks_file() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("nodaysidle-browser").join("bookmarks.json"))
}

pub fn default_bookmarks() -> Vec<Bookmark> {
    vec![
        Bookmark {
            title: "Gemini".into(),
            url: "https://gemini.google.com".into(),
        },
        Bookmark {
            title: "Google".into(),
            url: "https://google.com".into(),
        },
        Bookmark {
            title: "GitHub".into(),
            url: "https://github.com".into(),
        },
        Bookmark {
            title: "YouTube".into(),
            url: "https://youtube.com".into(),
        },
    ]
}

pub fn load_bookmarks() -> Vec<Bookmark> {
    let Some(path) = bookmarks_file() else {
        return default_bookmarks();
    };

    if !path.exists() {
        let defaults = default_bookmarks();
        save_bookmarks(&defaults);
        return defaults;
    }

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| default_bookmarks()),
        Err(_) => default_bookmarks(),
    }
}

pub fn save_bookmarks(bookmarks: &[Bookmark]) {
    let Some(path) = bookmarks_file() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(json) = serde_json::to_string_pretty(bookmarks) {
        let _ = fs::write(path, json);
    }
}

pub fn is_bookmarked(url: &str) -> bool {
    let norm = normalize_url(url);
    if norm.is_empty() || norm == "about:blank" {
        return false;
    }
    load_bookmarks()
        .iter()
        .any(|b| normalize_url(&b.url) == norm)
}

pub fn toggle_bookmark(title: &str, url: &str) -> bool {
    let norm = normalize_url(url);
    if norm.is_empty() || norm == "about:blank" {
        return false;
    }

    let mut bookmarks = load_bookmarks();
    if let Some(idx) = bookmarks.iter().position(|b| normalize_url(&b.url) == norm) {
        bookmarks.remove(idx);
        save_bookmarks(&bookmarks);
        false
    } else {
        bookmarks.push(Bookmark {
            title: if title.trim().is_empty() {
                url.to_string()
            } else {
                title.trim().to_string()
            },
            url: url.to_string(),
        });
        save_bookmarks(&bookmarks);
        true
    }
}

pub fn remove_bookmark(url: &str) {
    let norm = normalize_url(url);
    let mut bookmarks = load_bookmarks();
    bookmarks.retain(|b| normalize_url(&b.url) != norm);
    save_bookmarks(&bookmarks);
}

fn normalize_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("https://example.com/"), "https://example.com");
        assert_eq!(
            normalize_url("  HTTPS://Example.COM  "),
            "https://example.com"
        );
    }

    #[test]
    fn test_default_bookmarks_has_expected_entries() {
        let bms = default_bookmarks();
        assert!(!bms.is_empty());
        assert!(bms.iter().any(|b| b.title == "Gemini"));
    }
}
