use gio::prelude::*;
use gtk::prelude::*;
use gtk::{CssProvider, StyleContext};
use std::sync::OnceLock;

/// Fallback inset when compositor rounding cannot be detected (pixels).
/// Edge-flush on Linux (0px) to prevent fake macOS floating-island inner margins.
pub const WINDOW_CORNER_INSET_PX: i32 = 0;

/// `hyprctl` is spawned at most once per process: the decoration rounding never
/// changes while the browser runs, and the old code shelled out on startup and
/// on every `is-maximized` notification, which could block the main loop.
#[allow(dead_code)]
static HYPRLAND_ROUNDING: OnceLock<Option<i32>> = OnceLock::new();

const STYLESHEET: &str = r#"
/* Colours are tokens from DARK_PALETTE or PAPER_PALETTE below. Dialogs
   (permissions, downloads, About, file chooser) keep the GTK theme, which
   `apply` switches to dark or light to match (R-9, I-3). */
window.browser-window {
  background-color: @bg;
}

/* Popovers, menus and dialogs use the same palette, so Paper never shows
   the GTK theme's pure white and Dark never mixes in a lighter grey. */
popover.background,
menu,
.menu,
.context-menu,
dialog,
messagedialog {
  background-color: @surface;
  color: @text;
}

popover list,
popover row,
dialog list {
  background-color: transparent;
  color: @text;
}

popover entry,
dialog entry {
  background-color: @bg;
  color: @text;
  border-color: @border;
}

menu menuitem:hover,
.menu menuitem:hover {
  background-color: alpha(@accent, 0.18);
  color: @text_strong;
}

.browser-frame {
  background-color: @bg;
}

window.browser-window.maximized .browser-frame,
window.browser-window.tiled .browser-frame {
  border-radius: 0;
}

.chrome {
  background-color: @chrome;
}

.void {
  background-color: @bg;
}

.chrome-separator {
  background-color: @line;
  min-height: 1px;
}

.tab-strip {
  padding: 4px 8px 0;
}

/* The transparent border keeps selection from shifting the pill. */
.tab-pill {
  background-color: transparent;
  border-radius: 4px 4px 0 0;
  border-bottom: 2px solid transparent;
  padding: 2px 4px 0;
}

.tab-pill.selected {
  background-color: @selected;
  border-bottom-color: @accent;
}

.tab-pill-label {
  color: @muted;
  font-size: 12px;
  padding: 4px 8px;
}

.tab-pill-label-selected {
  color: @text_strong;
  font-weight: 600;
}

.tab-focus-list,
.tab-focus-list > .tab-focusable,
.tab-focus-list > .tab-focusable:selected {
  background-color: transparent;
  padding: 0;
}

.tab-focus-list > .tab-focusable:focus {
  background-color: alpha(@accent, 0.28);
  box-shadow: inset 0 0 0 1px @accent;
  border-radius: 4px;
}

/* Opacity, not visibility: the slot stays reserved so hovering never resizes
   the pill or moves its neighbours. */
.tab-close {
  color: @muted;
  padding: 2px;
  min-width: 20px;
  min-height: 20px;
  opacity: 0;
  transition: opacity 120ms ease-out;
}

.tab-pill.hover .tab-close,
.tab-pill.selected .tab-close {
  opacity: 1;
}

.tab-close:hover {
  color: @text;
  background-color: @hairline;
  border-radius: 4px;
}

.tab-new-btn:focus,
.ghost-btn:focus {
  background-color: alpha(@accent, 0.28);
  box-shadow: inset 0 0 0 1px @accent;
  color: @text_strong;
}

.tab-new-btn {
  color: @muted;
  background: transparent;
  border: none;
  min-width: 26px;
  min-height: 26px;
  border-radius: 999px;
}

.tab-new-btn:hover {
  background-color: @surface;
}

.ghost-btn {
  color: @muted;
  background: transparent;
  border: none;
  min-width: 30px;
  min-height: 30px;
  border-radius: 999px;
  padding: 0;
}

.ghost-btn:hover {
  background-color: @surface;
  color: @text_hover;
}

.ghost-btn:disabled {
  color: alpha(@muted, 0.4);
}

.url-bar {
  background-color: @surface;
  color: @text;
  /* 3:1 against the toolbar so the field edge is visible (WCAG 1.4.11). */
  border: 1px solid @border;
  border-radius: 8px;
  padding: 7px 12px;
  font-size: 13px;
}

.url-bar image.left {
  color: @muted;
}

.url-bar.secure image.left {
  color: @secure;
}

.url-bar.insecure image.left {
  color: @insecure;
}

.url-bar progress {
  background-color: transparent;
  background-image: linear-gradient(to top, @accent 2px, transparent 2px);
  border: none;
  border-radius: 0;
  box-shadow: none;
  margin: 0;
  padding: 0;
  min-width: 0;
}

.url-bar:focus {
  border-color: @accent;
  box-shadow: 0 0 0 1px alpha(@accent, 0.55);
}

.browser-toolbar {
  padding: 4px 8px;
}

.home-title {
  color: @text;
  letter-spacing: 0.04em;
}

.home-search-pill {
  background-color: @surface;
  border: 1px solid @border;
  border-radius: 999px;
  padding: 4px 16px;
}

.home-hint {
  color: @muted;
  font-size: 12px;
}

.home-search-pill.focused {
  border-color: @accent;
  box-shadow: 0 0 0 1px alpha(@accent, 0.55);
}

/* Bookmarks styling */
.star-btn {
  color: @muted;
}

.star-btn:hover {
  color: @text_strong;
  background-color: @surface;
}

.star-btn.bookmarked {
  color: @accent;
}

.bookmarks-bar {
  padding: 3px 8px;
  background-color: @chrome;
}

.bookmark-pill {
  background-color: transparent;
  color: @text;
  font-size: 12px;
  border-radius: 4px;
  padding: 2px 8px;
  border: 1px solid transparent;
}

.bookmark-pill:hover {
  background-color: @surface;
  border-color: @hairline;
  color: @text_strong;
}

/* Page assistant side panel. */
.assistant-panel {
  background-color: @surface;
  border-left: 1px solid @line;
  padding: 12px 14px;
}

.assistant-title {
  color: @text_strong;
  font-weight: 700;
  font-size: 14px;
}

.assistant-notice {
  color: @muted;
  font-size: 11px;
}

.assistant-btn {
  background-color: @chrome;
  color: @text;
  border: 1px solid @line;
  border-radius: 6px;
  padding: 4px 10px;
  font-size: 12px;
  font-weight: 500;
}

.assistant-btn:hover {
  background-color: @surface;
  border-color: @hairline;
  color: @text_strong;
}

.assistant-output,
.assistant-output text {
  background-color: @chrome;
  color: @text;
  font-size: 13px;
  border-radius: 6px;
  padding: 6px;
}

.find-bar {
  padding: 4px 8px;
  min-width: 0;
}

.find-entry {
  background-color: @surface;
  color: @text;
  border: 1px solid @border;
  border-radius: 8px;
  box-shadow: none;
  padding: 4px 10px;
  font-size: 13px;
  min-width: 0;
}

.find-entry:focus {
  border-color: @accent;
  box-shadow: 0 0 0 1px alpha(@accent, 0.55);
}

.find-entry.find-found {
  border-color: @secure;
}

.find-entry.find-none {
  border-color: @error;
}

.find-status {
  color: @muted;
  font-size: 12px;
  min-width: 0;
}

.home-search-entry,
.home-search-entry:focus {
  background: transparent;
  border: none;
  box-shadow: none;
  color: @text;
  font-size: 14px;
}
"#;

pub fn window_corner_inset_px() -> i32 {
    // The environment override stays dynamic so tests and runtime tweaks can
    // change it.
    if let Ok(raw) = std::env::var("NODAYSIDLE_CORNER_INSET") {
        if let Ok(value) = raw.parse::<i32>() {
            if value >= 0 {
                return value;
            }
        }
    }
    WINDOW_CORNER_INSET_PX
}

/// Reads Hyprland `decoration:rounding` when `hyprctl` is available.
#[allow(dead_code)]
fn hyprland_rounding_px() -> Option<i32> {
    // Only a Hyprland session sets this; skip the spawn everywhere else.
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
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

/// Decides the frame inset from the window's edge-flush state. A maximized,
/// fullscreen or tiled window has no compositor rounding to keep the chrome
/// away from, so its frame must sit flush (0). Pure so it can be unit-tested
/// without a display.
fn effective_frame_inset(
    default_inset: i32,
    maximized: bool,
    fullscreen: bool,
    gdk_state: gdk::WindowState,
    tiled_class: bool,
    maximized_class: bool,
) -> i32 {
    let edge_flush = maximized
        || fullscreen
        || tiled_class
        || maximized_class
        || gdk_state.intersects(
            gdk::WindowState::TILED | gdk::WindowState::MAXIMIZED | gdk::WindowState::FULLSCREEN,
        );
    if edge_flush {
        0
    } else {
        default_inset
    }
}

/// The inset this realization of `window` should use right now.
fn frame_inset_for_window(window: &gtk::Window) -> i32 {
    let gdk_state = window
        .window()
        .map(|gdk_window| gdk_window.state())
        .unwrap_or_else(gdk::WindowState::empty);
    let style = window.style_context();
    effective_frame_inset(
        window_corner_inset_px(),
        window.is_maximized(),
        // GTK 3 has no `is_fullscreen`; the GDK window state is authoritative.
        gdk_state.contains(gdk::WindowState::FULLSCREEN),
        gdk_state,
        style.has_class("tiled"),
        style.has_class("maximized"),
    )
}

fn sync_frame_margins(window: &gtk::Window, frame: &gtk::Box) {
    set_frame_margins(frame, frame_inset_for_window(window));
}

/// Keeps chrome flush to the screen edge while maximized, fullscreen or tiled
/// (Hyprland tiles are not `is_maximized()`, so the GDK window state and the
/// `tiled` style class are checked as well).
pub fn wire_window_corner_insets(window: &gtk::Window, frame: &gtk::Box) {
    sync_frame_margins(window, frame);
    let frame_for_notify = frame.clone();
    window.connect_notify_local(Some("is-maximized"), move |window, _| {
        sync_frame_margins(window, &frame_for_notify);
    });
    let frame_for_state = frame.clone();
    window.connect_window_state_event(move |window, _| {
        sync_frame_margins(window, &frame_for_state);
        glib::Propagation::Proceed
    });
}

/// Colour tokens the stylesheet uses. Dark is the original look; Paper is a
/// warm off-white for daylight, never pure white, and every text pair meets
/// WCAG AA (4.5:1) on the surfaces it sits on.
const DARK_PALETTE: &str = "
@define-color bg #151518;
@define-color chrome #1a1a1b;
@define-color surface #202022;
@define-color selected #242427;
@define-color line rgba(255, 255, 255, 0.09);
@define-color hairline rgba(255, 255, 255, 0.07);
@define-color text_strong #f2f2f5;
@define-color text #e8e8ec;
@define-color text_hover #d1d1d6;
@define-color muted #949499;
@define-color placeholder_text_color #949499;
@define-color border #6a6a70;
@define-color accent #75aaff;
@define-color secure #7fc8a0;
@define-color insecure #e5a46a;
@define-color error #e06c75;
";

const PAPER_PALETTE: &str = "
@define-color bg #f3efe6;
@define-color chrome #ebe6db;
@define-color surface #f8f5ee;
@define-color selected #ddd6c8;
@define-color line rgba(60, 50, 30, 0.14);
@define-color hairline rgba(60, 50, 30, 0.08);
@define-color text_strong #1f1d1a;
@define-color text #2b2925;
@define-color text_hover #3a3731;
@define-color muted #5e594f;
@define-color placeholder_text_color #5e594f;
@define-color border #8a8376;
@define-color accent #2c5aa0;
@define-color secure #2d6e47;
@define-color insecure #9a5212;
@define-color error #b3261e;
";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeChoice {
    System,
    Dark,
    Paper,
}

impl ThemeChoice {
    fn key(self) -> &'static str {
        match self {
            ThemeChoice::System => "system",
            ThemeChoice::Dark => "dark",
            ThemeChoice::Paper => "paper",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        [ThemeChoice::System, ThemeChoice::Dark, ThemeChoice::Paper]
            .into_iter()
            .find(|choice| choice.key() == key)
    }
}

/// Reads active Omarchy theme colors from ~/.config/omarchy/current_theme
/// and loads the corresponding colors.toml.
pub fn load_omarchy_palette() -> Option<(String, bool)> {
    let home = dirs::home_dir()?;
    let current_theme_file = home.join(".config/omarchy/current_theme");
    let slug = std::fs::read_to_string(&current_theme_file)
        .ok()?
        .trim()
        .to_string();
    if slug.is_empty() {
        return None;
    }

    let user_theme = home.join(format!(".config/omarchy/themes/{slug}/colors.toml"));
    let sys_theme =
        std::path::PathBuf::from(format!("/usr/share/omarchy/themes/{slug}/colors.toml"));

    let path = if user_theme.exists() {
        user_theme
    } else if sys_theme.exists() {
        sys_theme
    } else {
        return None;
    };

    let raw = std::fs::read_to_string(&path).ok()?;
    let mut map = std::collections::HashMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || !trimmed.contains('=') {
            continue;
        }
        let mut parts = trimmed.splitn(2, '=');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            let key = k.trim().to_string();
            let val = v
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .trim()
                .to_string();
            map.insert(key, val);
        }
    }

    let mode = map
        .get("mode")
        .cloned()
        .unwrap_or_else(|| "dark".to_string());
    let is_dark = mode != "light";

    let bg = map.get("background")?.clone();
    let chrome = map
        .get("darker_background")
        .or_else(|| map.get("dark_background"))
        .unwrap_or(&bg)
        .clone();
    let surface = map.get("lighter_background").unwrap_or(&bg).clone();
    let selected = map.get("selection").unwrap_or(&surface).clone();
    let text = map
        .get("foreground")
        .unwrap_or(&"#e8e8ec".to_string())
        .clone();
    let text_strong = map.get("bright_foreground").unwrap_or(&text).clone();
    let muted = map
        .get("muted")
        .or_else(|| map.get("dark_foreground"))
        .unwrap_or(&"#949499".to_string())
        .clone();
    let accent = map.get("accent").unwrap_or(&"#75aaff".to_string()).clone();
    let secure = map.get("green").unwrap_or(&"#7fc8a0".to_string()).clone();
    let insecure = map
        .get("yellow")
        .or_else(|| map.get("orange"))
        .unwrap_or(&"#e5a46a".to_string())
        .clone();
    let error = map.get("red").unwrap_or(&"#e06c75".to_string()).clone();

    let (line, hairline) = if is_dark {
        ("rgba(255, 255, 255, 0.09)", "rgba(255, 255, 255, 0.07)")
    } else {
        ("rgba(0, 0, 0, 0.12)", "rgba(0, 0, 0, 0.08)")
    };

    let css = format!(
        r#"
@define-color bg {bg};
@define-color chrome {chrome};
@define-color surface {surface};
@define-color selected {selected};
@define-color line {line};
@define-color hairline {hairline};
@define-color text_strong {text_strong};
@define-color text {text};
@define-color text_hover {text_strong};
@define-color muted {muted};
@define-color placeholder_text_color {muted};
@define-color border {selected};
@define-color accent {accent};
@define-color secure {secure};
@define-color insecure {insecure};
@define-color error {error};
"#
    );
    Some((css, is_dark))
}

thread_local! {
    static THEME: std::cell::RefCell<ThemeState> = std::cell::RefCell::new(ThemeState::default());
}

#[derive(Default)]
struct ThemeState {
    palette: Option<CssProvider>,
    /// CSS source of `palette`, read by `palette_colour`.
    palette_source: Option<String>,
    choice: Option<ThemeChoice>,
    dark: bool,
    settings_file: Option<std::path::PathBuf>,
    /// org.gnome.desktop.interface, when installed, for "Match system".
    interface: Option<gio::Settings>,
    /// The user's GTK theme name at startup, e.g. "Adwaita-dark".
    gtk_theme: Option<String>,
    /// Active file monitor for Omarchy theme changes
    omarchy_monitor: Option<gio::FileMonitor>,
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
    let interface = gio::SettingsSchemaSource::default()
        .and_then(|source| source.lookup("org.gnome.desktop.interface", true))
        .filter(|schema| schema.has_key("color-scheme"))
        .map(|_| gio::Settings::new("org.gnome.desktop.interface"));
    if let Some(interface) = &interface {
        interface.connect_changed(Some("color-scheme"), |_, _| {
            if current_choice() == ThemeChoice::System {
                apply(ThemeChoice::System);
            }
        });
    }

    // Monitor Omarchy active theme file for live theme sync
    let omarchy_monitor = dirs::home_dir().and_then(|home| {
        let theme_file = home.join(".config/omarchy/current_theme");
        let file = gio::File::for_path(&theme_file);
        file.monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
            .ok()
    });

    if let Some(ref monitor) = omarchy_monitor {
        monitor.connect_changed(|_, _, _, _| {
            glib::idle_add_local_once(|| {
                if current_choice() == ThemeChoice::System {
                    apply(ThemeChoice::System);
                }
            });
        });
    }

    THEME.with(|theme| {
        let mut t = theme.borrow_mut();
        t.interface = interface;
        t.omarchy_monitor = omarchy_monitor;
    });
    apply(ThemeChoice::System);
}

/// Loads the saved choice from `<profile>/settings.json` and remembers where
/// to save later changes.
pub fn load_preference(profile_dir: &std::path::Path) {
    let file = profile_dir.join("settings.json");
    let saved = std::fs::read_to_string(&file)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|json| json.get("theme")?.as_str().and_then(ThemeChoice::from_key))
        .unwrap_or(ThemeChoice::System);
    THEME.with(|theme| theme.borrow_mut().settings_file = Some(file));
    apply(saved);
}

pub fn current_choice() -> ThemeChoice {
    THEME.with(|theme| theme.borrow().choice.unwrap_or(ThemeChoice::System))
}

/// True while the dark palette is active (built-in pages follow it).
pub fn is_dark() -> bool {
    THEME.with(|theme| theme.borrow().dark)
}

/// The value of a colour token in `palette`.
fn palette_token<'a>(palette: &'a str, name: &str) -> Option<&'a str> {
    palette.lines().find_map(|line| {
        line.trim()
            .strip_prefix("@define-color ")?
            .strip_prefix(name)?
            .strip_prefix(' ')?
            .strip_suffix(';')
    })
}

/// A colour token of the active palette, so built-in pages match the chrome
/// under any theme, including Omarchy ones. Paper until a theme is applied.
pub fn palette_colour(name: &str) -> String {
    THEME.with(|theme| {
        let theme = theme.borrow();
        let palette = theme.palette_source.as_deref().unwrap_or(PAPER_PALETTE);
        palette_token(palette, name).unwrap_or_default().to_string()
    })
}

/// Switches theme now and saves the choice for the next launch.
pub fn choose(choice: ThemeChoice) {
    apply(choice);
    let file = THEME.with(|theme| theme.borrow().settings_file.clone());
    if let Some(file) = file {
        let json = serde_json::json!({ "theme": choice.key() }).to_string();
        if let Err(err) = std::fs::write(&file, json) {
            eprintln!("Could not save theme to {}: {err}", file.display());
        }
    }
}

fn system_prefers_dark(interface: Option<&gio::Settings>) -> bool {
    // Only an explicit light preference turns Paper on; "default" or no
    // desktop setting keeps the original dark look.
    interface.is_none_or(|settings| settings.string("color-scheme") != "prefer-light")
}

/// "Adwaita-dark" -> "Adwaita"; names without a dark suffix are unchanged.
fn light_variant(theme: &str) -> String {
    ["-dark", "-Dark", "_dark", ":dark"]
        .iter()
        .find_map(|suffix| theme.strip_suffix(suffix))
        .unwrap_or(theme)
        .to_string()
}

fn apply(choice: ThemeChoice) {
    let (source, dark) = match choice {
        ThemeChoice::Dark => (DARK_PALETTE.to_string(), true),
        ThemeChoice::Paper => (PAPER_PALETTE.to_string(), false),
        ThemeChoice::System => {
            if let Some((omarchy_css, is_dark)) = load_omarchy_palette() {
                (omarchy_css, is_dark)
            } else {
                let dark =
                    THEME.with(|theme| system_prefers_dark(theme.borrow().interface.as_ref()));
                let src = if dark { DARK_PALETTE } else { PAPER_PALETTE };
                (src.to_string(), dark)
            }
        }
    };
    let palette = CssProvider::new();
    if palette.load_from_data(source.as_bytes()).is_err() {
        return;
    }
    let Some(screen) = gdk::Screen::default() else {
        return;
    };
    // The palette must outrank the stylesheet's own priority so the tokens
    // resolve; swapping it restyles every open window at once.
    StyleContext::add_provider_for_screen(
        &screen,
        &palette,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 2,
    );
    let previous = THEME.with(|theme| {
        let mut theme = theme.borrow_mut();
        theme.choice = Some(choice);
        theme.dark = dark;
        theme.palette_source = Some(source);
        theme.palette.replace(palette)
    });
    if let Some(previous) = previous {
        StyleContext::remove_provider_for_screen(&screen, &previous);
    }
    // Dialogs use the GTK theme and WebKit derives prefers-color-scheme from
    // it, so both follow the browser's choice instead of mixing light and
    // dark. A theme named "*-dark" stays dark whatever prefer-dark says, so
    // Paper switches to its light variant.
    if let Some(settings) = gtk::Settings::default() {
        let original = THEME.with(|theme| {
            theme
                .borrow_mut()
                .gtk_theme
                .get_or_insert_with(|| settings.gtk_theme_name().unwrap_or_default().to_string())
                .clone()
        });
        let name = if dark {
            original.clone()
        } else {
            light_variant(&original)
        };
        if !name.is_empty() && settings.gtk_theme_name().as_deref() != Some(name.as_str()) {
            settings.set_gtk_theme_name(Some(&name));
        }
        settings.set_gtk_application_prefer_dark_theme(dark);
    }
    crate::error_page::follow_theme();
}

/// Called from the GTK test: the computed styles the chrome relies on (F02).
#[cfg(test)]
pub(crate) fn verify_chrome_styling() {
    use gtk::StateFlags;
    fn rgb(rgba: gdk::RGBA) -> (u8, u8, u8) {
        let channel = |value: f64| (value * 255.0).round() as u8;
        (
            channel(rgba.red()),
            channel(rgba.green()),
            channel(rgba.blue()),
        )
    }
    install();
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let pill = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    pill.style_context().add_class("tab-pill");
    let close = gtk::Button::new();
    close.style_context().add_class("tab-close");
    pill.pack_start(&close, false, false, 0);
    let url = gtk::Entry::new();
    url.style_context().add_class("url-bar");
    root.add(&pill);
    root.add(&url);
    window.add(&root);
    window.show_all();

    choose(ThemeChoice::Dark);
    let style = pill.style_context();
    let border = |style: &StyleContext| style.border(StateFlags::NORMAL);
    let bottom = |style: &StyleContext| {
        style
            .style_property_for_state("border-bottom-color", StateFlags::NORMAL)
            .get::<gdk::RGBA>()
            .unwrap()
    };
    let radius = |style: &StyleContext| {
        style
            .style_property_for_state("border-radius", StateFlags::NORMAL)
            .get::<i32>()
            .unwrap()
    };
    assert_eq!(border(&style).bottom, 2);
    assert_eq!(radius(&style), 4);
    assert_eq!(bottom(&style).alpha(), 0.0);
    // CSS opacity, which `WidgetExt::opacity` does not report.
    let close_opacity = close
        .style_context()
        .style_property_for_state("opacity", StateFlags::NORMAL)
        .get::<f64>()
        .unwrap();
    assert_eq!(close_opacity, 0.0);
    style.add_class("selected");
    // Selection adds the accent without changing the border box.
    assert_eq!(border(&style).bottom, 2);
    assert_eq!(rgb(bottom(&style)), (0x75, 0xaa, 0xff));
    assert_eq!(
        rgb(url
            .style_context()
            .lookup_color("placeholder_text_color")
            .unwrap()),
        (0x94, 0x94, 0x99)
    );
    choose(ThemeChoice::Paper);
    assert_eq!(rgb(bottom(&style)), (0x2c, 0x5a, 0xa0));
    assert_eq!(palette_colour("bg"), "#f3efe6");
    assert_eq!(
        rgb(url
            .style_context()
            .lookup_color("placeholder_text_color")
            .unwrap()),
        (0x5e, 0x59, 0x4f)
    );
    choose(ThemeChoice::Dark);
    assert_eq!(palette_colour("bg"), "#151518");
    window.close();
}

/// Called from the GTK test: switching theme restyles live widgets, the
/// GTK theme follows, and the choice is saved for the next launch.
#[cfg(test)]
pub(crate) fn verify_live_theme_switching() {
    install();
    let dir = std::env::temp_dir().join(format!("nodaysidle-theme-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    load_preference(&dir);
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.style_context().add_class("browser-window");
    let hint = gtk::Label::new(Some("hint"));
    hint.style_context().add_class("home-hint");
    window.add(&hint);
    window.show_all();
    let color = |label: &gtk::Label| {
        let rgba = label.style_context().color(gtk::StateFlags::NORMAL);
        (
            (rgba.red() * 255.0).round() as u8,
            (rgba.green() * 255.0).round() as u8,
            (rgba.blue() * 255.0).round() as u8,
        )
    };
    choose(ThemeChoice::Dark);
    assert!(is_dark());
    assert_eq!(color(&hint), (0x94, 0x94, 0x99));
    choose(ThemeChoice::Paper);
    assert!(!is_dark());
    assert_eq!(color(&hint), (0x5e, 0x59, 0x4f));
    let settings = gtk::Settings::default().unwrap();
    assert!(!settings.is_gtk_application_prefer_dark_theme());
    assert!(!settings
        .gtk_theme_name()
        .unwrap_or_default()
        .ends_with("-dark"));
    let saved = std::fs::read_to_string(dir.join("settings.json")).unwrap();
    assert!(saved.contains("\"paper\""));
    choose(ThemeChoice::Dark);
    assert_eq!(color(&hint), (0x94, 0x94, 0x99));
    window.close();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[cfg(test)]
mod tests {
    use super::{
        effective_frame_inset, light_variant, palette_token, parse_hyprland_rounding_json,
        DARK_PALETTE, PAPER_PALETTE,
    };

    #[test]
    fn palette_tokens_are_read_by_their_exact_name() {
        assert_eq!(palette_token(DARK_PALETTE, "bg"), Some("#151518"));
        assert_eq!(palette_token(PAPER_PALETTE, "text"), Some("#2b2925"));
        // "text" must not match "text_strong".
        assert_eq!(
            palette_token("@define-color text_strong #000;", "text"),
            None
        );
        assert_eq!(palette_token(PAPER_PALETTE, "missing"), None);
    }

    #[test]
    fn paper_uses_the_light_variant_of_the_gtk_theme() {
        assert_eq!(light_variant("Adwaita-dark"), "Adwaita");
        assert_eq!(light_variant("Yaru-Dark"), "Yaru");
        assert_eq!(light_variant("Adwaita"), "Adwaita");
    }

    #[test]
    fn hyprland_rounding_json_is_parsed_from_hyprctl_output() {
        assert_eq!(
            parse_hyprland_rounding_json(br#"{"int":20,"set":true}"#),
            Some(20)
        );
        assert_eq!(
            parse_hyprland_rounding_json(br#"{"int":0,"set":true}"#),
            Some(0)
        );
        assert_eq!(parse_hyprland_rounding_json(b"not json"), None);
        assert_eq!(parse_hyprland_rounding_json(br#"{"set":true}"#), None);
    }

    #[test]
    fn frame_inset_is_zero_when_the_window_is_edge_flush() {
        use gdk::WindowState;

        // Normal floating window: keep the compositor rounding inset.
        assert_eq!(
            effective_frame_inset(18, false, false, WindowState::empty(), false, false),
            18
        );

        // `is_maximized()` alone is enough.
        assert_eq!(
            effective_frame_inset(18, true, false, WindowState::empty(), false, false),
            0
        );
        // Fullscreen is enough too.
        assert_eq!(
            effective_frame_inset(18, false, true, WindowState::empty(), false, false),
            0
        );
        // Hyprland tiles are not `is_maximized()`: the GDK state must count.
        assert_eq!(
            effective_frame_inset(18, false, false, WindowState::TILED, false, false),
            0
        );
        assert_eq!(
            effective_frame_inset(18, false, false, WindowState::MAXIMIZED, false, false),
            0
        );
        assert_eq!(
            effective_frame_inset(18, false, false, WindowState::FULLSCREEN, false, false),
            0
        );
        // The style class signal counted as well.
        assert_eq!(
            effective_frame_inset(18, false, false, WindowState::empty(), true, false),
            0
        );
        assert_eq!(
            effective_frame_inset(18, false, false, WindowState::empty(), false, true),
            0
        );
        // An inset of 0 stays 0 regardless.
        assert_eq!(
            effective_frame_inset(0, false, false, WindowState::empty(), false, false),
            0
        );
    }
}
