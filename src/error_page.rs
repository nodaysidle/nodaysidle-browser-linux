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

const LOAD_FAILED_TITLE: &str = "This page could not be loaded";
const PROCESS_ENDED_TITLE: &str = "This page stopped working";

/// True for the title of one of these pages, so they stay out of history.
pub fn is_error_page_title(title: Option<&str>) -> bool {
    matches!(title, Some(LOAD_FAILED_TITLE | PROCESS_ENDED_TITLE))
}

pub fn load_failed_html(uri: &str, message: &str) -> String {
    page(LOAD_FAILED_TITLE, uri, &[uri, message], "Try again")
}

pub fn web_process_ended_html(uri: &str, exceeded_memory: bool) -> String {
    let reason = if exceeded_memory {
        "The page used too much memory and was stopped."
    } else {
        "The process showing this page crashed."
    };
    page(PROCESS_ENDED_TITLE, uri, &[uri, reason], "Reload")
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
  border: 1px solid rgba(255,255,255,0.14); border-radius: 999px;
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
    use super::{html_escape, is_error_page_title, load_failed_html, web_process_ended_html};

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
        assert!(html.contains("https://x.test/?q=&lt;b&gt;"));
        assert!(!html.contains("<b>"));
        assert!(html.contains("Could not connect: refused"));
        assert!(html.contains("location.replace(&quot;https://x.test/?q=%3Cb%3E&quot;)"));
        assert!(html.contains("background: #151518"));

        let crashed = web_process_ended_html("https://example.com/", false);
        assert!(crashed.contains("crashed"));
        assert!(crashed.contains(">Reload</button>"));
        assert!(web_process_ended_html("https://example.com/", true).contains("memory"));
        assert!(is_error_page_title(Some("This page could not be loaded")));
        assert!(!is_error_page_title(Some("Example Domain")));
        // No retry button for URLs a page must not be able to smuggle in.
        assert!(!load_failed_html("javascript:alert(1)", "x").contains("<button"));
    }
}
