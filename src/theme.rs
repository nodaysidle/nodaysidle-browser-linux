use gtk::prelude::*;
use gtk::{CssProvider, StyleContext};

const STYLESHEET: &str = r#"
window {
  background-color: #151518;
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
  padding: 4px 8px 4px 12px;
}

.tab-pill {
  background-color: transparent;
  border-radius: 8px;
  padding: 2px 4px;
  margin: 0 2px;
}

.tab-pill.selected {
  background-color: #202022;
}

.tab-pill-label {
  color: #949499;
  font-family: monospace;
  font-size: 11px;
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
  color: #6b6b70;
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

.url-bar:focus {
  border-color: #75aaff;
  box-shadow: 0 0 0 1px rgba(117, 170, 255, 0.55);
}

.toolbar {
  padding: 4px 10px;
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

.find-bar {
  padding: 4px 10px;
}

.find-entry {
  background-color: #202022;
  color: #e8e8ec;
  border: 1px solid rgba(255, 255, 255, 0.09);
  border-radius: 8px;
  padding: 4px 10px;
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
}

.home-search-entry {
  background: transparent;
  border: none;
  color: #e8e8ec;
  font-size: 14px;
}
"#;

pub fn install() {
    let provider = CssProvider::new();
    if provider
        .load_from_data(STYLESHEET.as_bytes())
        .is_err()
    {
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
