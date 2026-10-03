/// URL / search resolution aligned with the macOS nodaysidle-browser app.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchEngine {
    DuckDuckGo,
}

impl Default for SearchEngine {
    fn default() -> Self {
        SearchEngine::DuckDuckGo
    }
}

impl SearchEngine {
    pub fn search_url(&self, query: &str) -> Option<String> {
        let encoded = percent_encode_query(query);
        match self {
            SearchEngine::DuckDuckGo => {
                Some(format!("https://duckduckgo.com/?q={}", encoded))
            }
        }
    }
}

pub fn resolve(raw: &str, engine: SearchEngine) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Some(trimmed.to_string());
    }

    if trimmed.contains(' ') {
        return engine.search_url(trimmed);
    }

    if is_loopback_target(trimmed) {
        return Some(format!("http://{}", trimmed));
    }

    if trimmed.contains('.') {
        return Some(format!("https://{}", trimmed));
    }

    engine.search_url(trimmed)
}

pub fn title_for_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            if !host.is_empty() {
                return host.to_string();
            }
        }
        if !parsed.path().is_empty() {
            return parsed.to_string();
        }
    }
    if url.is_empty() {
        "New Tab".to_string()
    } else {
        url.to_string()
    }
}

pub fn title_for_page(title: Option<&str>, url: &str) -> String {
    title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| title_for_url(url))
}

fn is_loopback_target(input: &str) -> bool {
    let host = url::Url::parse(&format!("http://{}", input))
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()));

    match host.as_deref() {
        Some("localhost") => true,
        Some(h) if h.ends_with(".localhost") => true,
        Some("127.0.0.1") => true,
        Some("::1") | Some("[::1]") => true,
        _ => false,
    }
}

fn percent_encode_query(query: &str) -> String {
    let mut out = String::new();
    for ch in query.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '.' | '_' | '~') {
            out.push(ch);
        } else {
            for byte in ch.to_string().as_bytes() {
                out.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::title_for_page;

    #[test]
    fn empty_page_titles_fall_back_to_the_page_url() {
        assert_eq!(
            title_for_page(Some(""), "https://example.com/a"),
            "example.com"
        );
        assert_eq!(
            title_for_page(Some("  "), "https://example.com/a"),
            "example.com"
        );
        assert_eq!(title_for_page(None, "https://example.com/a"), "example.com");
    }

    #[test]
    fn nonempty_page_titles_are_preserved_without_padding() {
        assert_eq!(
            title_for_page(Some(" Page title "), "https://example.com"),
            "Page title"
        );
    }
}
