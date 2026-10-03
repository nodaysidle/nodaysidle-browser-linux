/// URL / search resolution aligned with the macOS nodaysidle-browser app.
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};

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

/// Turns address-bar input into a URL, or into a search when it does not look
/// like one (X-15).
pub fn resolve(raw: &str, engine: SearchEngine) -> Option<String> {
    resolve_with_home(raw, engine, dirs::home_dir().as_deref())
}

fn resolve_with_home(raw: &str, engine: SearchEngine, home: Option<&Path>) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Explicit schemes the browser loads directly. Anything else that merely
    // parses as `scheme:` (e.g. `localhost:3000`, `javascript:`) is handled
    // below or searched.
    let lower = trimmed.to_ascii_lowercase();
    if ["http://", "https://", "file://", "about:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
    {
        return Some(trimmed.to_string());
    }

    if let Some(path) = local_path(trimmed, home) {
        return url::Url::from_file_path(&path).ok().map(String::from);
    }

    if trimmed.chars().any(char::is_whitespace) {
        return engine.search_url(trimmed);
    }

    // Unbracketed IPv6 literal such as `::1` or `fe80::1`.
    if let Ok(address) = trimmed.parse::<Ipv6Addr>() {
        let scheme = if is_local_ipv6(address) { "http" } else { "https" };
        return Some(format!("{scheme}://[{address}]/"));
    }

    match host_kind(trimmed) {
        HostKind::Local => Some(format!("http://{trimmed}")),
        HostKind::Public => Some(format!("https://{trimmed}")),
        HostKind::NotAHost => engine.search_url(trimmed),
    }
}

/// Absolute (`/etc/hostname`) and home-relative (`~/page.html`) paths.
fn local_path(input: &str, home: Option<&Path>) -> Option<PathBuf> {
    if input.starts_with('/') {
        return Some(PathBuf::from(input));
    }
    let rest = input.strip_prefix("~/")?;
    Some(home?.join(rest))
}

#[derive(Debug, PartialEq, Eq)]
enum HostKind {
    /// Loopback, private-network or single-label host with a port: plain http.
    Local,
    Public,
    NotAHost,
}

/// File extensions that are not top-level domains, so `node.js` or
/// `notes.txt` is searched instead of opened as `https://node.js`.
const NON_TLD_SUFFIXES: &[&str] = &[
    "js", "mjs", "ts", "tsx", "jsx", "txt", "json", "html", "htm", "css", "scss", "exe", "dll",
    "pdf", "png", "jpg", "jpeg", "gif", "svg", "webp", "zip", "tar", "gz", "xz", "toml", "yaml",
    "yml", "xml", "csv", "log", "cpp", "hpp", "java", "kt", "swift", "lock", "conf", "ini", "cfg",
    "deb", "rpm", "iso", "mp3", "mp4", "mkv", "wav", "doc", "docx", "xls", "xlsx", "odt",
];

fn host_kind(input: &str) -> HostKind {
    let Ok(parsed) = url::Url::parse(&format!("http://{input}")) else {
        return HostKind::NotAHost;
    };
    // Reject anything that did not stay in the host part, such as user info.
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return HostKind::NotAHost;
    }
    let raw_host = raw_host(input);
    match parsed.host() {
        Some(url::Host::Ipv6(address)) if is_local_ipv6(address) => HostKind::Local,
        Some(url::Host::Ipv6(_)) => HostKind::Public,
        Some(url::Host::Ipv4(_)) => {
            // WHATWG parsing turns `3.14` into 3.0.0.14; only a full dotted
            // quad typed by the user counts as an address.
            match raw_host.parse::<Ipv4Addr>() {
                Ok(address) if is_local_ipv4(address) => HostKind::Local,
                Ok(_) => HostKind::Public,
                Err(_) => HostKind::NotAHost,
            }
        }
        Some(url::Host::Domain(domain)) => {
            let domain = domain.to_ascii_lowercase();
            if domain == "localhost" || domain.ends_with(".localhost") {
                return HostKind::Local;
            }
            match domain.rsplit_once('.') {
                Some((_, tld)) if is_plausible_tld(tld) => HostKind::Public,
                Some(_) => HostKind::NotAHost,
                // `intranet:8080`: a single label is only a host with a port.
                None if parsed.port().is_some() => HostKind::Local,
                None => HostKind::NotAHost,
            }
        }
        None => HostKind::NotAHost,
    }
}

/// Loopback (127.0.0.0/8), private (RFC 1918: 10/8, 172.16/12, 192.168/16)
/// and link-local (169.254/16) addresses: local services that rarely have a
/// certificate, so they get http.
fn is_local_ipv4(address: Ipv4Addr) -> bool {
    address.is_loopback() || address.is_private() || address.is_link_local()
}

/// Loopback (::1), unique local (fc00::/7), link-local (fe80::/10) and
/// IPv4-mapped local addresses (V-5).
fn is_local_ipv6(address: Ipv6Addr) -> bool {
    address.is_loopback()
        || address.is_unique_local()
        || address.is_unicast_link_local()
        || address.to_ipv4_mapped().is_some_and(is_local_ipv4)
}

/// Host part of `host[:port][/path]` as typed.
fn raw_host(input: &str) -> &str {
    let end = input.find(['/', '?', '#']).unwrap_or(input.len());
    let authority = &input[..end];
    authority
        .rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
        .map_or(authority, |(host, _)| host)
}

fn is_plausible_tld(tld: &str) -> bool {
    if let Some(punycode) = tld.strip_prefix("xn--") {
        return !punycode.is_empty();
    }
    tld.len() >= 2
        && tld.chars().all(|c| c.is_ascii_alphabetic())
        && !NON_TLD_SUFFIXES.contains(&tld)
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
    use super::{resolve_with_home, title_for_page, SearchEngine};
    use std::path::Path;

    fn resolve(input: &str) -> String {
        resolve_with_home(input, SearchEngine::DuckDuckGo, Some(Path::new("/home/me")))
            .expect("non-empty input resolves")
    }

    fn is_search(url: &str) -> bool {
        url.starts_with("https://duckduckgo.com/?q=")
    }

    #[test]
    fn explicit_supported_schemes_are_loaded_as_typed() {
        assert_eq!(resolve("https://example.com/a"), "https://example.com/a");
        assert_eq!(resolve("HTTP://Example.com"), "HTTP://Example.com");
        assert_eq!(resolve("about:blank"), "about:blank");
        assert_eq!(resolve("file:///etc/hostname"), "file:///etc/hostname");
        assert_eq!(resolve("file:///home/me/a.b/c.d"), "file:///home/me/a.b/c.d");
    }

    #[test]
    fn script_and_data_urls_are_searched_not_loaded() {
        assert!(is_search(&resolve("javascript:alert(1)")));
        assert!(is_search(&resolve("data:text/html,hi")));
    }

    #[test]
    fn absolute_and_home_paths_become_file_urls() {
        assert_eq!(resolve("/etc/hostname"), "file:///etc/hostname");
        assert_eq!(resolve("/tmp/my page.html"), "file:///tmp/my%20page.html");
        assert_eq!(resolve("~/docs/a.html"), "file:///home/me/docs/a.html");
        assert!(resolve_with_home("~/a.html", SearchEngine::DuckDuckGo, None)
            .is_some_and(|url| is_search(&url)));
    }

    #[test]
    fn ipv6_literals_are_bracketed() {
        assert_eq!(resolve("::1"), "http://[::1]/");
        assert_eq!(resolve("[::1]:8080/x"), "http://[::1]:8080/x");
        assert_eq!(resolve("2001:db8::1"), "https://[2001:db8::1]/");
    }

    #[test]
    fn private_and_link_local_ipv6_use_http() {
        assert_eq!(resolve("fd00::1"), "http://[fd00::1]/");
        assert_eq!(resolve("fc00::5"), "http://[fc00::5]/");
        assert_eq!(resolve("fe80::1"), "http://[fe80::1]/");
        assert_eq!(resolve("[fd12:3456::1]:8080/admin"), "http://[fd12:3456::1]:8080/admin");
        assert_eq!(resolve("[fe80::abcd]/"), "http://[fe80::abcd]/");
        assert_eq!(resolve("::ffff:192.168.1.1"), "http://[::ffff:192.168.1.1]/");
        assert_eq!(resolve("fec0::1"), "https://[fec0::1]/");
        assert_eq!(resolve("[2606:4700::1111]"), "https://[2606:4700::1111]");
    }

    #[test]
    fn local_and_private_hosts_use_http() {
        assert_eq!(resolve("localhost:3000"), "http://localhost:3000");
        assert_eq!(resolve("app.localhost"), "http://app.localhost");
        assert_eq!(resolve("127.0.0.1:8011/a"), "http://127.0.0.1:8011/a");
        assert_eq!(resolve("192.168.1.1"), "http://192.168.1.1");
        assert_eq!(resolve("10.0.0.2:8080"), "http://10.0.0.2:8080");
        assert_eq!(resolve("172.16.0.1"), "http://172.16.0.1");
        assert_eq!(resolve("172.31.255.254/x"), "http://172.31.255.254/x");
        assert_eq!(resolve("169.254.10.20"), "http://169.254.10.20");
        assert_eq!(resolve("172.32.0.1"), "https://172.32.0.1");
        assert_eq!(resolve("intranet:8080"), "http://intranet:8080");
    }

    #[test]
    fn domains_use_https() {
        assert_eq!(resolve("wikipedia.org"), "https://wikipedia.org");
        assert_eq!(resolve("en.wikipedia.org/wiki/Rust"), "https://en.wikipedia.org/wiki/Rust");
        assert_eq!(resolve("example.com:8443"), "https://example.com:8443");
        assert_eq!(resolve("8.8.8.8"), "https://8.8.8.8");
        assert_eq!(resolve("пример.рф"), "https://пример.рф");
    }

    #[test]
    fn file_names_numbers_and_words_are_searched() {
        for input in ["node.js", "notes.txt", "3.14", "rust", "e.g", "user@example.com"] {
            assert!(is_search(&resolve(input)), "{input} should be a search");
        }
        assert!(is_search(&resolve("how to use example.com")));
    }

    #[test]
    fn blank_input_does_nothing() {
        assert_eq!(resolve_with_home("   ", SearchEngine::DuckDuckGo, None), None);
    }

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
