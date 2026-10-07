//! Spoof-resistant URL presentation for address fields, tooltips, and titles.
//! Canonical navigation URIs are unchanged; only human-facing text is sanitized.

use url::Url;

/// User-visible URL text. The underlying `uri` used for navigation must stay canonical.
pub fn format_url_for_display(uri: &str) -> String {
    match Url::parse(uri) {
        Ok(parsed) => {
            let rendered = if let Some(host) = parsed.host_str() {
                let shown = display_host(host);
                if shown != host {
                    url_with_display_host(&parsed, &shown)
                } else {
                    parsed.as_ref().to_string()
                }
            } else {
                parsed.as_ref().to_string()
            };
            sanitize_percent_utf8_in_display(&rendered)
        }
        Err(_) => sanitize_percent_utf8_in_display(uri),
    }
}

fn url_with_display_host(parsed: &Url, host: &str) -> String {
    let mut out = format!("{}://{}", parsed.scheme(), host);
    if let Some(port) = parsed.port() {
        out.push(':');
        out.push_str(&port.to_string());
    }
    out.push_str(parsed.path());
    if let Some(query) = parsed.query() {
        out.push('?');
        out.push_str(query);
    }
    if let Some(fragment) = parsed.fragment() {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

/// Host portion for fallback tab titles and tooltips.
pub fn display_host_for_title(host: &str) -> String {
    display_host(host)
}

fn display_host(host: &str) -> String {
    if host.is_empty() {
        return host.to_string();
    }
    if host_labels_safe_for_unicode_display(host) {
        if host.contains("xn--") || !host.is_ascii() {
            idna::domain_to_unicode(host).0
        } else {
            host.to_string()
        }
    } else {
        idna::domain_to_ascii(host).unwrap_or_else(|_| host.to_string())
    }
}

fn host_labels_safe_for_unicode_display(host: &str) -> bool {
    let (unicode, _) = idna::domain_to_unicode(host);
    unicode.split('.').all(|label| {
        !label.is_empty()
            && !label.chars().any(is_unsafe_display_char)
            && !label_has_mixed_scripts(label)
    })
}

/// Latin letters together with letters from another script in the same label.
fn label_has_mixed_scripts(label: &str) -> bool {
    let mut has_latin = false;
    let mut has_other_letter = false;
    for ch in label.chars() {
        if !ch.is_alphabetic() {
            continue;
        }
        if ch.is_ascii() && ch.is_ascii_alphabetic() {
            has_latin = true;
        } else {
            has_other_letter = true;
        }
    }
    has_latin && has_other_letter
}

fn is_unsafe_display_char(ch: char) -> bool {
    ch.is_control()
        || matches!(
            ch,
            '\u{00ad}' // soft hyphen
                | '\u{061c}' // arabic letter mark
                | '\u{180e}' // mongolian vowel separator
                | '\u{200b}'..='\u{200f}' // ZWSP..RLM
                | '\u{202a}'..='\u{202e}' // bidi embedding/override
                | '\u{2060}'..='\u{2069}' // word joiner, bidi isolates
                | '\u{feff}' // BOM / ZWNBSP
        )
}

fn sanitize_percent_utf8_in_display(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let mut decoded_bytes = [0u8; 4];
            let mut byte_count = 0;
            let mut idx = i;
            let mut decoded_char = None;
            let mut consumed = 0;

            while idx + 2 < bytes.len() && bytes[idx] == b'%' && byte_count < 4 {
                let hex = &bytes[idx + 1..idx + 3];
                let Ok(hex_str) = std::str::from_utf8(hex) else {
                    break;
                };
                let Ok(b) = u8::from_str_radix(hex_str, 16) else {
                    break;
                };
                if b < 0x80 {
                    break;
                }
                decoded_bytes[byte_count] = b;
                byte_count += 1;
                idx += 3;

                if let Ok(s) = std::str::from_utf8(&decoded_bytes[..byte_count]) {
                    let mut chars = s.chars();
                    if let Some(ch) = chars.next() {
                        if chars.next().is_none() && !is_unsafe_display_char(ch) {
                            decoded_char = Some(ch);
                            consumed = idx - i;
                            break;
                        }
                    }
                }
            }

            if let Some(ch) = decoded_char {
                out.push(ch);
                i += consumed;
                continue;
            }
        }
        let ch = input[i..].chars().next().unwrap();
        if is_unsafe_display_char(ch) {
            for byte in ch.to_string().as_bytes() {
                out.push_str(&format!("%{:02X}", byte));
            }
        } else {
            out.push(ch);
        }
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{display_host_for_title, format_url_for_display};

    #[test]
    fn punycode_hosts_decode_when_labels_are_single_script() {
        let host = "xn--e1afmkfd.xn--80akhbyknj4f";
        assert_eq!(display_host_for_title(host), "пример.испытание");
    }

    #[test]
    fn legitimate_international_paths_and_domains_still_decode() {
        assert_eq!(
            format_url_for_display(
                "https://zh.wikipedia.org/wiki/%E7%BB%B4%E5%9F%BA%E7%99%BE%E7%A7%91"
            ),
            "https://zh.wikipedia.org/wiki/维基百科"
        );
        assert_eq!(
            format_url_for_display(
                "https://de.wikipedia.org/wiki/M%C3%BCnchen?q=test%20space%26more"
            ),
            "https://de.wikipedia.org/wiki/München?q=test%20space%26more"
        );
        assert_eq!(
            format_url_for_display(
                "https://xn--e1afmkfd.xn--80akhbyknj4f/wiki/%E7%BB%B4%E5%9F%BA%E7%99%BE%E7%A7%91"
            ),
            "https://пример.испытание/wiki/维基百科"
        );
        assert_eq!(display_host_for_title("пример.рф"), "пример.рф");
    }

    #[test]
    fn mixed_script_idn_stays_in_punycode() {
        // Cyrillic "р" (U+0430) followed by Latin letters in the same label.
        let shown = format_url_for_display("https://\u{0440}oypal.com/");
        assert!(
            shown.contains("xn--"),
            "mixed-script label should not be shown as Unicode: {shown}"
        );
    }

    #[test]
    fn bidi_and_zero_width_stay_percent_encoded_or_literal_safe() {
        let bidi = format_url_for_display("https://example.com/%E2%80%AEevil");
        assert!(
            !bidi.contains('\u{202e}'),
            "U+202E must not appear decoded in display: {bidi}"
        );
        assert!(bidi.contains("%E2%80%AE") || bidi.contains("%e2%80%ae"));
        let zw = format_url_for_display("https://example.com/a%E2%80%8Bb");
        assert!(!zw.contains('\u{200b}'));
    }

    #[test]
    fn ascii_delimiters_in_percent_encoding_remain_encoded() {
        assert_eq!(
            format_url_for_display("https://example.com/foo%20bar%2Fbaz%3Fq%3D1"),
            "https://example.com/foo%20bar%2Fbaz%3Fq%3D1"
        );
    }

    #[test]
    fn about_and_file_urls_follow_the_same_rules() {
        assert_eq!(format_url_for_display("about:blank"), "about:blank");
        assert_eq!(
            format_url_for_display("file:///home/%E6%96%87%E6%A1%A3/test.html"),
            "file:///home/文档/test.html"
        );
    }
}
