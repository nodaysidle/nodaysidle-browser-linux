//! Application identity: icon name, embedded icon fallback (X-26).

use gtk::gdk_pixbuf::{Pixbuf, PixbufLoader};
use gtk::prelude::*;

/// Themed icon name installed by scripts/install-desktop.sh.
pub const ICON_NAME: &str = "nodaysidle-browser";

const ICON_SVG: &[u8] = include_bytes!("../assets/icon.svg");

/// Gives every window the app icon (_NET_WM_ICON on X11). Uses the themed
/// icon when it is installed and the embedded SVG otherwise, e.g. when the
/// binary runs straight from target/.
pub fn install_default_icon() {
    if theme_has_icon() {
        gtk::Window::set_default_icon_name(ICON_NAME);
        return;
    }
    let icons: Vec<Pixbuf> = [16, 32, 48, 128, 256]
        .into_iter()
        .filter_map(embedded_icon)
        .collect();
    if !icons.is_empty() {
        gtk::Window::set_default_icon_list(&icons);
    }
}

pub fn theme_has_icon() -> bool {
    gtk::IconTheme::default().is_some_and(|theme| theme.has_icon(ICON_NAME))
}

/// The embedded icon rendered at `size` px, if gdk-pixbuf can load SVG.
pub fn embedded_icon(size: i32) -> Option<Pixbuf> {
    let loader = PixbufLoader::with_type("svg").ok()?;
    loader.set_size(size, size);
    loader.write(ICON_SVG).ok()?;
    loader.close().ok()?;
    loader.pixbuf()
}
