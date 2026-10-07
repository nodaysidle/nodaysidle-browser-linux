use gtk::prelude::*;
use gtk::{CssProvider, StyleContext};

/// Fallback inset when compositor rounding cannot be detected (pixels).
pub const WINDOW_CORNER_INSET_PX: i32 = 8;

const STYLESHEET: &str = r#"
/* Only the browser's own windows are dark; dialogs (permissions, downloads,
   About, file chooser) keep the GTK theme's colours (R-9, I-3). */
window.browser-window {
  background-color: #151518;
}

.browser-frame {
  background-color: #151518;
}

window.browser-window.maximized .browser-frame,
window.browser-window.tiled .browser-frame {
  border-radius: 0;
}

.chrome {
  background-color: #1a1a1b;
}

.void {
  background-color: #151518;
}

.chrome-separator {
  background-color: rgba(255, 255, 255, 0.09);
  min-height: 1px;
}

.tab-strip {
  padding: 4px 8px;
}

.tab-pill {
  background-color: transparent;
  border-radius: 8px;
  padding: 2px 4px;
  margin: 0 2px;
}

.tab-pill.selected {
  background-color: #202022;
  box-shadow: inset 0 -2px 0 #75aaff;
}

.tab-pill-label {
  color: #949499;
  font-family: monospace;
  font-size: 12px;
  padding: 4px 8px;
}

.tab-pill-label-selected {
  color: #e8e8ec;
}

.tab-focusable:focus {
  background-color: rgba(116, 170, 255, 0.28);
  box-shadow: inset 0 0 0 1px #75aaff;
  border-radius: 6px;
}

.tab-close {
  color: #8a8a90;
  padding: 2px;
  min-width: 20px;
  min-height: 20px;
}

.tab-close:hover {
  color: #e8e8ec;
}

.tab-new-btn:focus,
.ghost-btn:focus {
  background-color: rgba(116, 170, 255, 0.28);
  box-shadow: inset 0 0 0 1px #75aaff;
  color: #ffffff;
}

.tab-new-btn {
  color: #949499;
  background: transparent;
  border: none;
  min-width: 26px;
  min-height: 26px;
  border-radius: 999px;
}

.tab-new-btn:hover {
  background-color: #202022;
}

.ghost-btn {
  color: #949499;
  background: transparent;
  border: none;
  min-width: 30px;
  min-height: 30px;
  border-radius: 999px;
  padding: 0;
}

.ghost-btn:hover {
  background-color: #202022;
  color: #d1d1d6;
}

.ghost-btn:disabled {
  color: rgba(107, 107, 112, 0.35);
}

.url-bar {
  background-color: #202022;
  color: #e8e8ec;
  border: 1px solid rgba(255, 255, 255, 0.09);
  border-radius: 8px;
  padding: 7px 12px;
  font-size: 13px;
}

.url-bar.secure image.left {
  color: #7fc8a0;
}

.url-bar.insecure image.left {
  color: #e5a46a;
}

.url-bar progress {
  background-color: transparent;
  background-image: linear-gradient(to top, #75aaff 2px, transparent 2px);
  border: none;
  border-radius: 0;
  box-shadow: none;
  margin: 0;
  padding: 0;
  min-width: 0;
}

.url-bar:focus {
  border-color: #75aaff;
  box-shadow: 0 0 0 1px rgba(117, 170, 255, 0.55);
}

.toolbar {
  padding: 4px 8px;
}

.home-title {
  color: #e8e8ec;
  letter-spacing: 0.04em;
}

.home-search-pill {
  background-color: #202022;
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 999px;
  padding: 4px 16px;
}

.home-search-pill.focused {
  border-color: #75aaff;
  box-shadow: 0 0 0 1px rgba(117, 170, 255, 0.55);
}

.find-bar {
  padding: 4px 8px;
  min-width: 0;
}

.find-entry {
  background-color: #202022;
  color: #e8e8ec;
  border: 1px solid rgba(255, 255, 255, 0.09);
  border-radius: 8px;
  padding: 4px 10px;
  min-width: 0;
}

.find-entry:focus {
  border-color: #75aaff;
}

.find-entry.find-none {
  border-color: #e06c75;
}

.find-status {
  color: #949499;
  font-size: 12px;
  min-width: 0;
}

.home-search-entry,
.home-search-entry:focus {
  background: transparent;
  border: none;
  box-shadow: none;
  color: #e8e8ec;
  font-size: 14px;
}
"#;

pub fn window_corner_inset_px() -> i32 {
    if let Ok(raw) = std::env::var("NODAYSIDLE_CORNER_INSET") {
        if let Ok(value) = raw.parse::<i32>() {
            if value >= 0 {
                return value;
            }
        }
    }
    hyprland_rounding_px().unwrap_or(WINDOW_CORNER_INSET_PX)
}

/// Reads Hyprland `decoration:rounding` when `hyprctl` is available.
fn hyprland_rounding_px() -> Option<i32> {
    let output = std::process::Command::new("hyprctl")
        .args(["-j", "getoption", "decoration:rounding"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_hyprland_rounding_json(&output.stdout)
}

fn parse_hyprland_rounding_json(bytes: &[u8]) -> Option<i32> {
    let parsed: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    parsed
        .get("int")
        .and_then(|value| value.as_i64())
        .map(|value| value as i32)
}

fn set_frame_margins(frame: &gtk::Box, inset: i32) {
    frame.set_margin_start(inset);
    frame.set_margin_end(inset);
    frame.set_margin_top(inset);
    frame.set_margin_bottom(inset);
}

/// Pads the main chrome away from compositor-rounded window corners.
pub fn apply_browser_frame_insets(frame: &gtk::Box, css_id: &str) {
    let inset = window_corner_inset_px();
    set_frame_margins(frame, inset);
    if inset <= 0 {
        return;
    }
    frame.set_widget_name(css_id);
    let chrome_radius = inset.min(12);
    let css = format!(
        r#"
box#{css_id}.browser-frame {{
  border-radius: {inset}px;
}}
box#{css_id}.browser-frame .chrome.tab-strip,
box#{css_id}.browser-frame .chrome.popup-chrome {{
  border-top-left-radius: {chrome_radius}px;
  border-top-right-radius: {chrome_radius}px;
}}
"#,
        css_id = css_id,
        inset = inset,
        chrome_radius = chrome_radius,
    );
    let provider = CssProvider::new();
    if provider.load_from_data(css.as_bytes()).is_ok() {
        frame
            .style_context()
            .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
    }
}

fn sync_frame_margins(window: &gtk::Window, frame: &gtk::Box) {
    let inset = if window.is_maximized() {
        0
    } else {
        window_corner_inset_px()
    };
    set_frame_margins(frame, inset);
}

/// Keeps chrome flush to the screen edge while maximized (Hyprland uses rounding 0).
pub fn wire_window_corner_insets(window: &gtk::Window, frame: &gtk::Box) {
    sync_frame_margins(window, frame);
    let frame = frame.clone();
    window.connect_notify_local(Some("is-maximized"), move |window, _| {
        sync_frame_margins(window, &frame);
    });
}

pub fn install() {
    let provider = CssProvider::new();
    if provider.load_from_data(STYLESHEET.as_bytes()).is_err() {
        return;
    }
    if let Some(screen) = gdk::Screen::default() {
        StyleContext::add_provider_for_screen(
            &screen,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::parse_hyprland_rounding_json;

    #[test]
    fn hyprland_rounding_json_is_parsed_from_hyprctl_output() {
        assert_eq!(
            parse_hyprland_rounding_json(br#"{"int":20,"set":true}"#),
            Some(20)
        );
        assert_eq!(parse_hyprland_rounding_json(b"not json"), None);
    }
}
