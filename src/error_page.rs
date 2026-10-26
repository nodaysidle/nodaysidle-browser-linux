//! Built-in pages shown in place of a page that failed to load (X-21) or
//! whose web process went away (X-27). They match the browser's dark chrome.

/// Escapes text for use in HTML element content and quoted attributes.
pub fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

// Short enough to read in a tab pill.
const LOAD_FAILED_TITLE: &str = "Could not load page";
const PROCESS_ENDED_TITLE: &str = "Page stopped working";
const TLS_FAILED_TITLE: &str = "Connection not secure";

/// True for the title of one of these pages, so they stay out of history.
pub fn is_error_page_title(title: Option<&str>) -> bool {
    matches!(
        title,
        Some(LOAD_FAILED_TITLE | PROCESS_ENDED_TITLE | TLS_FAILED_TITLE)
    )
}

pub fn load_failed_html(uri: &str, message: &str) -> String {
    let shown_uri = crate::navigation::format_url_for_display(uri);
    page(LOAD_FAILED_TITLE, uri, &[&shown_uri, message], "Try again")
}

/// Certificate problems get their own page instead of the generic failure:
/// the user needs to know the site may not be who it claims to be. There is
/// deliberately no "continue anyway".
pub fn tls_failed_html(uri: &str, problems: &[&str]) -> String {
    let shown_uri = crate::navigation::format_url_for_display(uri);
    let reason = if problems.is_empty() {
        "The site's certificate could not be verified.".to_string()
    } else {
        format!("The site's certificate {}.", problems.join(", "))
    };
    page(
        TLS_FAILED_TITLE,
        uri,
        &[
            &shown_uri,
            &reason,
            "Someone could be pretending to be this site. Nothing was sent to it.",
        ],
        "Try again",
    )
}

pub fn web_process_ended_html(uri: &str, exceeded_memory: bool) -> String {
    let reason = if exceeded_memory {
        "The page used too much memory and was stopped."
    } else {
        "The process showing this page crashed."
    };
    let shown_uri = crate::navigation::format_url_for_display(uri);
    page(PROCESS_ENDED_TITLE, uri, &[&shown_uri, reason], "Reload")
}

/// JavaScript that loads `uri` again, as an HTML-escaped attribute value.
/// `location.reload()` is not used: after a web process crash it reloads
/// about:blank instead of the page. Only web and file URLs get a button.
fn retry_script(uri: &str) -> Option<String> {
    let parsed = url::Url::parse(uri).ok()?;
    if !matches!(parsed.scheme(), "http" | "https" | "file") {
        return None;
    }
    let literal = serde_json::to_string(parsed.as_str()).ok()?;
    Some(html_escape(&format!("location.replace({literal})")))
}

fn page(heading: &str, uri: &str, details: &[&str], button: &str) -> String {
    let details = details
        .iter()
        .filter(|detail| !detail.is_empty())
        .map(|detail| format!("<p>{}</p>", html_escape(detail)))
        .collect::<String>();
    let button = retry_script(uri)
        .map(|script| {
            format!(
                r#"<button autofocus onclick="{script}">{}</button>"#,
                html_escape(button)
            )
        })
        .unwrap_or_default();
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>{heading}</title>
<style>
:root {{ color-scheme: dark; }}
body {{ margin: 0; min-height: 100vh; display: flex; align-items: center;
  justify-content: center; background: #151518; color: #e8e8ec;
  font: 15px system-ui, sans-serif; }}
main {{ max-width: 560px; padding: 32px; }}
h1 {{ font-size: 20px; font-weight: 600; margin: 0 0 12px; }}
p {{ color: #949499; margin: 6px 0; overflow-wrap: anywhere; }}
button {{ margin-top: 20px; background: #202022; color: #e8e8ec;
  border: 1px solid #6a6a70; border-radius: 999px;
  padding: 8px 20px; font: inherit; cursor: pointer; }}
button:hover, button:focus {{ border-color: #75aaff; outline: none; }}
</style></head>
<body><main><h1>{heading}</h1>{details}
{button}</main></body></html>"#,
        heading = html_escape(heading),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        html_escape, is_error_page_title, load_failed_html, tls_failed_html, web_process_ended_html,
    };

    #[test]
    fn html_escape_neutralises_markup_and_quotes() {
        assert_eq!(
            html_escape(r#"<script>"a" & 'b'</script>"#),
            "&lt;script&gt;&quot;a&quot; &amp; &#39;b&#39;&lt;/script&gt;"
        );
        assert_eq!(html_escape("Новости 维基"), "Новости 维基");
    }

    #[test]
    fn error_pages_show_the_escaped_url_and_reason_with_a_retry_button() {
        let html = load_failed_html("https://x.test/?q=<b>", "Could not connect: refused");
        assert!(html.contains("<p>https://x.test/?q=%3Cb%3E</p>"));
        assert!(!html.contains("<b>"));
        assert!(html.contains("Could not connect: refused"));
        assert!(html.contains("location.replace(&quot;https://x.test/?q=%3Cb%3E&quot;)"));
        assert!(html.contains("background: #151518"));

        let crashed = web_process_ended_html("https://example.com/", false);
        assert!(crashed.contains("crashed"));
        assert!(crashed.contains(">Reload</button>"));
        assert!(web_process_ended_html("https://example.com/", true).contains("memory"));
        assert!(is_error_page_title(Some("Could not load page")));
        let tls = tls_failed_html("https://bad.example/", &["has expired"]);
        assert!(tls.contains("<title>Connection not secure</title>"));
        assert!(tls.contains("certificate has expired."));
        assert!(!tls.to_lowercase().contains("continue"));
        assert!(!is_error_page_title(Some("Example Domain")));
        // No retry button for URLs a page must not be able to smuggle in.
        assert!(!load_failed_html("javascript:alert(1)", "x").contains("<button"));
    }

    #[test]
    fn error_url_presentation_keeps_retry_destinations_canonical() {
        let uri = "https://user:p%40ss@xn--bcher-kva.de:8443/%E2%80%AE?q=1&b=%2F#frag";
        for html in [
            load_failed_html(uri, "<failed>"),
            web_process_ended_html(uri, false),
        ] {
            assert!(html.contains("<p>https://bücher.de:8443/%E2%80%AE?q=1&amp;b=%2F#frag</p>"));
            assert!(html.contains(&format!(
                "location.replace(&quot;{}&quot;)",
                html_escape(uri)
            )));
            assert!(!html.contains('\u{202e}'));
        }
        assert!(load_failed_html("invalid <url> & test", "<failed>")
            .contains("<p>invalid &lt;url&gt; &amp; test</p>"));
    }
}
