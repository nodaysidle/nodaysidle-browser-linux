//! Page assistant: a scam check through TypeSafe Jev, and summarize or ask
//! about the current page through the Anthropic API (Claude Haiku 5.5).
//!
//! Like Brave Leo's "bring your own model" and extensions such as Page
//! Assist, keys come from the user (`TYPESAFE_API_KEY`, `ANTHROPIC_API_KEY`
//! in the environment, never written to disk), requests go straight from the
//! browser to the provider, and nothing is sent until the user clicks.

use crate::tabs::TabManagerInner;
use gtk::prelude::*;
use serde_json::{json, Value};
use soup::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use webkit2gtk::WebViewExt;

const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2026-10-21";
const JEV_URL: &str = "https://api.typesafe.ai/v1/systemone";
/// The cheapest current Claude model; summaries and page questions need no more.
const DEFAULT_MODEL: &str = "claude-haiku-5-5";
/// Page text sent with a question; about 3k tokens.
const PAGE_TEXT_LIMIT: usize = 12_000;
const PANEL_WIDTH: i32 = 340;

#[derive(Clone)]
pub struct AssistantPanel {
    pub(crate) revealer: gtk::Revealer,
    output: gtk::TextView,
    status: gtk::Label,
    entry: gtk::Entry,
    summarize: gtk::Button,
    scam_check: gtk::Button,
    stop: gtk::Button,
    close: gtk::Button,
    /// Conversation for follow-up questions, reset when the page changes.
    messages: Rc<RefCell<Vec<Value>>>,
    conversation_uri: Rc<RefCell<String>>,
    /// Bumped by Stop and by every new request, so a stale stream stops.
    generation: Rc<Cell<u64>>,
    session: soup::Session,
}

fn env_key(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|key| !key.trim().is_empty())
}

fn model() -> String {
    env_key("NODAYSIDLE_AI_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

pub(crate) fn build_panel() -> AssistantPanel {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.style_context().add_class("assistant-panel");
    panel.set_width_request(PANEL_WIDTH);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let title = gtk::Label::new(Some("Ask about this page"));
    title.style_context().add_class("assistant-title");
    title.set_xalign(0.0);
    let close = crate::tabs::icon_button("window-close-symbolic", "Close (Esc)");
    header.pack_start(&title, true, true, 0);
    header.pack_end(&close, false, false, 0);

    let claude = env_key("ANTHROPIC_API_KEY").is_some();
    let jev = env_key("TYPESAFE_API_KEY").is_some();
    // Always visible, so it is clear what leaves the machine and where to.
    let notice =
        gtk::Label::new(Some(&match (claude, jev) {
            (false, false) => "Set TYPESAFE_API_KEY (scam check) or ANTHROPIC_API_KEY (answers) \
                           in your environment and restart the browser."
                .to_string(),
            _ => {
                format!(
            "Sends this page's text only when you click: scam check to typesafe.ai (Jev{}), \
             questions to api.anthropic.com ({}).",
            if jev { "" } else { ", no key" },
            if claude { model() } else { "set ANTHROPIC_API_KEY".into() }
        )
            }
        }));
    notice.style_context().add_class("assistant-notice");
    notice.set_line_wrap(true);
    // GTK 3 only wraps a label once its width is capped.
    notice.set_max_width_chars(36);
    notice.set_xalign(0.0);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let summarize = gtk::Button::with_mnemonic("_Summarize");
    summarize.set_sensitive(claude);
    let scam_check = gtk::Button::with_mnemonic("Scam _check");
    scam_check.set_sensitive(jev);
    scam_check.set_tooltip_text(Some(
        "Ask Jev whether this page impersonates a brand, asks for secrets or pressures you",
    ));
    let stop = gtk::Button::with_label("Stop");
    stop.set_no_show_all(true);
    actions.pack_start(&summarize, false, false, 0);
    actions.pack_start(&scam_check, false, false, 0);
    actions.pack_end(&stop, false, false, 0);

    let output = gtk::TextView::new();
    output.set_editable(false);
    output.set_cursor_visible(false);
    output.set_wrap_mode(gtk::WrapMode::WordChar);
    output.set_left_margin(4);
    output.set_right_margin(4);
    output.style_context().add_class("assistant-output");
    let buffer = output.buffer().expect("a new TextView has a buffer");
    if let Some(table) = buffer.tag_table() {
        let you = gtk::TextTag::new(Some("you"));
        you.set_weight(700);
        table.add(&you);
        table.add(&gtk::TextTag::new(Some("error")));
    }

    let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_vexpand(true);
    scroll.add(&output);
    // GTK measures wrapped lines lazily, so follow the content height as it
    // grows instead of scrolling once per append (that left the last line cut).
    scroll.vadjustment().connect_changed(|adjustment| {
        adjustment.set_value(adjustment.upper() - adjustment.page_size())
    });

    let status = gtk::Label::new(None);
    status.style_context().add_class("assistant-notice");
    status.set_line_wrap(true);
    status.set_max_width_chars(36);
    status.set_xalign(0.0);

    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some(if claude {
        "Ask a question, then press Enter"
    } else {
        "Needs ANTHROPIC_API_KEY"
    }));
    entry.set_sensitive(claude);
    entry.style_context().add_class("find-entry");

    panel.pack_start(&header, false, false, 0);
    panel.pack_start(&notice, false, false, 0);
    panel.pack_start(&actions, false, false, 0);
    panel.pack_start(&scroll, true, true, 0);
    panel.pack_start(&status, false, false, 0);
    panel.pack_start(&entry, false, false, 0);

    let revealer = gtk::Revealer::new();
    revealer.set_transition_type(gtk::RevealerTransitionType::SlideLeft);
    revealer.add(&panel);
    revealer.set_reveal_child(false);
    // Hidden until opened; the window's show_all() must not reveal it.
    panel.show_all();
    revealer.set_no_show_all(true);

    let session = soup::Session::new();
    // Read timeout: a model may take a while before its first token.
    session.set_timeout(90);

    AssistantPanel {
        revealer,
        output,
        status,
        entry,
        summarize,
        scam_check,
        stop,
        close,
        messages: Rc::new(RefCell::new(Vec::new())),
        conversation_uri: Rc::new(RefCell::new(String::new())),
        generation: Rc::new(Cell::new(0)),
        session,
    }
}

pub(crate) fn toggle(mgr: &Rc<RefCell<TabManagerInner>>) {
    let panel = mgr.borrow().assistant.clone();
    if panel.revealer.reveals_child() {
        hide(&panel);
    } else {
        // At most half the window, so a narrow tile still shows the page.
        let window_width = { mgr.borrow().window.allocated_width() };
        if let Some(child) = panel.revealer.child() {
            child.set_width_request((window_width / 2).clamp(220, PANEL_WIDTH));
        }
        panel.revealer.show();
        panel.revealer.set_reveal_child(true);
        if panel.entry.is_sensitive() {
            panel.entry.grab_focus();
        } else {
            panel.close.grab_focus();
        }
    }
}

fn hide(panel: &AssistantPanel) {
    panel.revealer.set_reveal_child(false);
    panel.revealer.hide();
}

pub(crate) fn wire(mgr: &Rc<RefCell<TabManagerInner>>) {
    let panel = mgr.borrow().assistant.clone();
    panel.close.connect_clicked({
        let panel = panel.clone();
        move |_| hide(&panel)
    });
    panel.entry.connect_key_press_event({
        let panel = panel.clone();
        move |_, event| {
            if event.keyval() == gdk::keys::constants::Escape {
                hide(&panel);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    });
    panel.stop.connect_clicked({
        let panel = panel.clone();
        move |_| {
            panel.generation.set(panel.generation.get() + 1);
            finish(&panel, "Stopped.");
        }
    });
    panel.summarize.connect_clicked({
        let mgr = mgr.clone();
        move |_| {
            ask(
                &mgr,
                "Summarize this page in a few short paragraphs, then list its key points.",
            )
        }
    });
    panel.entry.connect_activate({
        let mgr = mgr.clone();
        move |entry| {
            let question = entry.text().trim().to_string();
            if !question.is_empty() {
                entry.set_text("");
                ask(&mgr, &question);
            }
        }
    });
    panel.scam_check.connect_clicked({
        let mgr = mgr.clone();
        move |_| scam_check(&mgr)
    });
}

fn append(panel: &AssistantPanel, text: &str, tag: Option<&str>) {
    let Some(buffer) = panel.output.buffer() else {
        return;
    };
    if let Some(error) = buffer.tag_table().and_then(|table| table.lookup("error")) {
        // No single red reaches 4.5:1 on both palettes' surfaces.
        let red = if crate::theme::is_dark() {
            "#e06c75"
        } else {
            "#b3261e"
        };
        error.set_foreground(Some(red));
    }
    let start = buffer.end_iter().offset();
    let mut end = buffer.end_iter();
    buffer.insert(&mut end, text);
    if let Some(tag) = tag {
        buffer.apply_tag_by_name(tag, &buffer.iter_at_offset(start), &buffer.end_iter());
    }
}

fn start(panel: &AssistantPanel, status: &str) -> u64 {
    let generation = panel.generation.get() + 1;
    panel.generation.set(generation);
    panel.status.set_text(status);
    panel.stop.show();
    panel.summarize.set_sensitive(false);
    panel.scam_check.set_sensitive(false);
    generation
}

fn finish(panel: &AssistantPanel, status: &str) {
    panel.status.set_text(status);
    panel.stop.hide();
    panel
        .summarize
        .set_sensitive(env_key("ANTHROPIC_API_KEY").is_some());
    panel
        .scam_check
        .set_sensitive(env_key("TYPESAFE_API_KEY").is_some());
}

struct PageText {
    uri: String,
    /// Origin and path only: query strings and fragments can hold tokens.
    shown_url: String,
    title: String,
    text: String,
    truncated: bool,
}

/// Reads the selection, or the page's main text, from the selected tab.
async fn page_text(mgr: &Rc<RefCell<TabManagerInner>>) -> Result<PageText, String> {
    let view = mgr
        .borrow()
        .selected_webview()
        .ok_or("Open a web page first.")?;
    let uri = view.uri().unwrap_or_default().to_string();
    let parsed = url::Url::parse(&uri).map_err(|_| "Open a web page first.".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Only web pages (http or https) are sent; local files stay private.".into());
    }
    let script = format!(
        "(() => {{ const sel = String(getSelection()).trim(); \
         const root = document.querySelector('article, main') || document.body; \
         const text = (sel || (root ? root.innerText : '')).replace(/\\n{{3,}}/g, '\\n\\n'); \
         return JSON.stringify({{ title: document.title, text: text.slice(0, {PAGE_TEXT_LIMIT}), \
         truncated: text.length > {PAGE_TEXT_LIMIT} }}); }})()"
    );
    let value = view
        .evaluate_javascript_future(&script, None, None)
        .await
        .map_err(|err| format!("Could not read the page: {err}"))?;
    let json: Value = serde_json::from_str(&javascriptcore::ValueExt::to_str(&value))
        .map_err(|_| "Could not read the page.".to_string())?;
    let text = json["text"].as_str().unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Err("This page has no readable text.".into());
    }
    Ok(PageText {
        shown_url: format!("{}{}", parsed.origin().ascii_serialization(), parsed.path()),
        uri,
        title: json["title"].as_str().unwrap_or_default().to_string(),
        text,
        truncated: json["truncated"].as_bool().unwrap_or(false),
    })
}

fn post(url: &str, headers: &[(&str, &str)], body: &Value) -> Result<soup::Message, String> {
    let message = soup::Message::new("POST", url).map_err(|err| err.to_string())?;
    if let Some(request_headers) = message.request_headers() {
        for (name, value) in headers {
            request_headers.append(name, value);
        }
    }
    let bytes = glib::Bytes::from_owned(body.to_string().into_bytes());
    message.set_request_body_from_bytes(Some("application/json"), Some(&bytes));
    Ok(message)
}

/// Turns an HTTP failure into something the user can act on.
fn http_error(provider: &str, code: u32, body: &str) -> String {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|json| json["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_default();
    let what = match code {
        401 => format!("{provider} rejected the API key"),
        402 => format!("Your {provider} account is out of credits"),
        403 => format!("{provider} refused this request"),
        408 | 504 => format!("{provider} timed out"),
        429 => format!("{provider} is rate limiting; try again in a moment"),
        502 | 503 | 529 => format!("The {provider} model is unavailable right now"),
        _ => format!("{provider} request failed"),
    };
    if detail.is_empty() {
        format!("{what} ({code}).")
    } else {
        format!("{what} ({code}): {detail}")
    }
}

/// One SSE line from the Messages API stream, as what the panel should do.
#[derive(Debug, PartialEq)]
enum StreamEvent {
    Text(String),
    Thinking,
    Done,
    Error(String),
    Ignore,
}

fn parse_stream_line(line: &str) -> StreamEvent {
    // `event:` names repeat the `type` field in the data, and blank lines
    // separate events, so only `data:` lines matter.
    let Some(data) = line.strip_prefix("data:").map(str::trim) else {
        return StreamEvent::Ignore;
    };
    let Ok(json) = serde_json::from_str::<Value>(data) else {
        return StreamEvent::Ignore;
    };
    match json["type"].as_str().unwrap_or_default() {
        "content_block_delta" => match json["delta"]["type"].as_str() {
            Some("text_delta") => {
                StreamEvent::Text(json["delta"]["text"].as_str().unwrap_or_default().into())
            }
            Some("thinking_delta") => StreamEvent::Thinking,
            _ => StreamEvent::Ignore,
        },
        "content_block_start" if json["content_block"]["type"] == "thinking" => {
            StreamEvent::Thinking
        }
        "message_delta" if json["delta"]["stop_reason"] == "refusal" => {
            StreamEvent::Error("Claude declined to answer this.".into())
        }
        "message_stop" => StreamEvent::Done,
        "error" => StreamEvent::Error(
            json["error"]["message"]
                .as_str()
                .unwrap_or("the stream failed")
                .into(),
        ),
        _ => StreamEvent::Ignore,
    }
}

fn ask(mgr: &Rc<RefCell<TabManagerInner>>, question: &str) {
    let panel = mgr.borrow().assistant.clone();
    let Some(key) = env_key("ANTHROPIC_API_KEY") else {
        return;
    };
    let generation = start(&panel, "Reading the page…");
    append(&panel, &format!("\n{question}\n"), Some("you"));
    let mgr = mgr.clone();
    let question = question.to_string();
    glib::MainContext::default().spawn_local(async move {
        let page = match page_text(&mgr).await {
            Ok(page) => page,
            Err(err) => {
                append(&panel, &format!("{err}\n"), Some("error"));
                finish(&panel, "");
                return;
            }
        };
        if *panel.conversation_uri.borrow() != page.uri {
            panel.messages.borrow_mut().clear();
            *panel.conversation_uri.borrow_mut() = page.uri.clone();
        }
        panel
            .messages
            .borrow_mut()
            .push(json!({ "role": "user", "content": question }));
        let system = format!(
            "You answer questions about the web page below for the person reading it. \
             Be concise and use plain text. If the page does not contain the answer, say so.\n\n\
             Title: {}\nURL: {}\n{}<page>\n{}\n</page>",
            page.title,
            page.shown_url,
            if page.truncated {
                "(The page text was cut to fit.)\n"
            } else {
                ""
            },
            page.text
        );
        let messages = panel.messages.borrow().clone();
        // Low effort: page Q&A is chat-like and stays cheap on Haiku 5.5.
        let body = json!({
            "model": model(),
            "max_tokens": 8000,
            "stream": true,
            "system": system,
            "messages": messages,
            "output_config": { "effort": "low" },
        });
        panel.status.set_text("Waiting for api.anthropic.com…");
        let answer = stream_answer(&panel, generation, &key, &body).await;
        if panel.generation.get() != generation {
            return;
        }
        match answer {
            Ok(answer) => {
                panel
                    .messages
                    .borrow_mut()
                    .push(json!({ "role": "assistant", "content": answer }));
                append(&panel, "\n", None);
                finish(
                    &panel,
                    if page.truncated {
                        "Done. Long page: only the first part was sent."
                    } else {
                        "Done."
                    },
                );
            }
            Err(err) => {
                // Keep the conversation consistent for the next question.
                panel.messages.borrow_mut().pop();
                append(&panel, &format!("\n{err}\n"), Some("error"));
                finish(&panel, "");
            }
        }
    });
}

async fn stream_answer(
    panel: &AssistantPanel,
    generation: u64,
    key: &str,
    body: &Value,
) -> Result<String, String> {
    let message = post(
        ANTHROPIC_URL,
        &[("x-api-key", key), ("anthropic-version", ANTHROPIC_VERSION)],
        body,
    )?;
    let stream = panel
        .session
        .send_future(&message, glib::Priority::DEFAULT)
        .await
        .map_err(|err| format!("Could not reach api.anthropic.com: {err}"))?;
    let lines = gio::DataInputStream::new(&stream);
    if message.status_code() != 200 {
        let mut body = String::new();
        while let Ok(Some(line)) = lines.read_line_utf8_future(glib::Priority::DEFAULT).await {
            body.push_str(&line);
        }
        return Err(http_error("Anthropic", message.status_code(), &body));
    }
    let mut answer = String::new();
    loop {
        if panel.generation.get() != generation {
            // Dropping the stream closes the connection, which cancels the request.
            return Err("Stopped.".into());
        }
        let line = match lines.read_line_utf8_future(glib::Priority::DEFAULT).await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(err) => return Err(format!("The connection to api.anthropic.com broke: {err}")),
        };
        match parse_stream_line(&line) {
            StreamEvent::Text(text) => {
                if answer.is_empty() {
                    panel.status.set_text("Answering…");
                }
                answer.push_str(&text);
                append(panel, &text, None);
            }
            StreamEvent::Thinking if answer.is_empty() => panel.status.set_text("Thinking…"),
            StreamEvent::Done => break,
            StreamEvent::Error(err) => return Err(format!("Anthropic: {err}")),
            StreamEvent::Thinking | StreamEvent::Ignore => {}
        }
    }
    if answer.is_empty() {
        return Err("The model returned no answer.".into());
    }
    Ok(answer)
}

/// Jev's three judgments, combined by explicit rules rather than one score.
fn scam_verdict(impersonation: f64, credentials: f64, pressure: f64) -> &'static str {
    if impersonation >= 0.7 && (credentials >= 0.5 || pressure >= 0.7) {
        "High risk: this looks like a scam. Do not enter passwords or payment details."
    } else if impersonation >= 0.7 || pressure >= 0.7 {
        "Be careful: some scam signals."
    } else {
        "No strong scam signals."
    }
}

fn scam_check(mgr: &Rc<RefCell<TabManagerInner>>) {
    let panel = mgr.borrow().assistant.clone();
    let Some(key) = env_key("TYPESAFE_API_KEY") else {
        return;
    };
    let generation = start(&panel, "Reading the page…");
    append(&panel, "\nScam check\n", Some("you"));
    let mgr = mgr.clone();
    glib::MainContext::default().spawn_local(async move {
        let page = match page_text(&mgr).await {
            Ok(page) => page,
            Err(err) => {
                append(&panel, &format!("{err}\n"), Some("error"));
                finish(&panel, "");
                return;
            }
        };
        let host = url::Url::parse(&page.uri)
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .unwrap_or_default();
        let body = json!({
            "model": "jev-latest",
            "state": { "host": host, "title": page.title, "text": page.text },
            "questions": {
                "impersonation": { "type": "noul", "instructions":
                    "Does the page in `state` present itself as a well-known brand, bank or \
                     service that its `host` does not belong to?" },
                "credentials": { "type": "noul", "instructions":
                    "Does the page in `state` ask the visitor for a password, one-time code, \
                     payment card or account recovery details?" },
                "pressure": { "type": "noul", "instructions":
                    "Does the page in `state` use threats, deadlines or alarming warnings to \
                     rush the visitor into acting?" }
            }
        });
        panel.status.set_text("Waiting for typesafe.ai…");
        let result = async {
            let auth = format!("Bearer {key}");
            let message = post(JEV_URL, &[("Authorization", &auth)], &body)?;
            let bytes = panel
                .session
                .send_and_read_future(&message, glib::Priority::DEFAULT)
                .await
                .map_err(|err| format!("Could not reach typesafe.ai: {err}"))?;
            let text = String::from_utf8_lossy(&bytes).to_string();
            if message.status_code() != 200 {
                return Err(http_error("TypeSafe", message.status_code(), &text));
            }
            serde_json::from_str::<Value>(&text).map_err(|_| "TypeSafe sent an unreadable reply.".to_string())
        }
        .await;
        if panel.generation.get() != generation {
            return;
        }
        match result {
            Ok(json) => {
                let p = |name: &str| json["answers"][name]["noul"].as_f64().unwrap_or(0.0);
                let (imp, cred, press) = (p("impersonation"), p("credentials"), p("pressure"));
                append(
                    &panel,
                    &format!(
                        "{}\nPretends to be another brand: {:.0}%\nAsks for passwords or card details: {:.0}%\nPressures you to act fast: {:.0}%\n(Jev's estimate for {host}; it can be wrong.)\n",
                        scam_verdict(imp, cred, press),
                        imp * 100.0,
                        cred * 100.0,
                        press * 100.0
                    ),
                    None,
                );
                finish(&panel, "Done.");
            }
            Err(err) => {
                append(&panel, &format!("{err}\n"), Some("error"));
                finish(&panel, "");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{http_error, parse_stream_line, scam_verdict, StreamEvent};

    #[test]
    fn messages_api_stream_lines_are_read_like_the_docs_describe() {
        assert_eq!(
            parse_stream_line("event: content_block_delta"),
            StreamEvent::Ignore
        );
        assert_eq!(parse_stream_line(""), StreamEvent::Ignore);
        assert_eq!(
            parse_stream_line(
                r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#
            ),
            StreamEvent::Text("Hello".into())
        );
        assert_eq!(
            parse_stream_line(
                r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#
            ),
            StreamEvent::Thinking
        );
        assert_eq!(
            parse_stream_line(r#"data: {"type":"message_stop"}"#),
            StreamEvent::Done
        );
        assert_eq!(
            parse_stream_line(
                r#"data: {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#
            ),
            StreamEvent::Error("Overloaded".into())
        );
        assert!(matches!(
            parse_stream_line(
                r#"data: {"type":"message_delta","delta":{"stop_reason":"refusal"},"usage":{"output_tokens":1}}"#
            ),
            StreamEvent::Error(_)
        ));
    }

    #[test]
    fn http_errors_say_what_to_do() {
        assert_eq!(
            http_error(
                "Anthropic",
                401,
                r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#
            ),
            "Anthropic rejected the API key (401): invalid x-api-key"
        );
        assert_eq!(
            http_error("TypeSafe", 429, ""),
            "TypeSafe is rate limiting; try again in a moment (429)."
        );
    }

    #[test]
    fn scam_verdict_needs_impersonation_plus_a_second_signal_for_high_risk() {
        assert!(scam_verdict(0.98, 0.9, 0.1).starts_with("High risk"));
        assert!(scam_verdict(0.98, 0.1, 0.1).starts_with("Be careful"));
        assert!(scam_verdict(0.1, 0.9, 0.1).starts_with("No strong"));
    }
}
