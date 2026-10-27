//! Keyboard shortcuts overview (Ctrl+?), the standard GTK shortcuts window.
//! Keep this list in sync with `tabs::shortcut_for`.

use gtk::prelude::*;

const SHORTCUTS: &[(&str, &[(&str, &str)])] = &[
    (
        "Tabs and windows",
        &[
            ("<Primary>t", "New tab"),
            ("<Primary><Shift>t", "Reopen closed tab"),
            ("<Primary>w", "Close tab"),
            ("<Primary>Tab <Primary>Page_Down", "Next tab"),
            ("<Primary><Shift>Tab <Primary>Page_Up", "Previous tab"),
            ("<Primary>1...8", "Go to tab by position"),
            ("<Primary>9", "Go to last tab"),
            ("<Primary>n", "New window"),
            ("<Primary>q", "Quit"),
        ],
    ),
    (
        "Navigation",
        &[
            ("<Primary>l <Alt>d F6", "Focus address bar"),
            ("Escape", "Leave the address bar"),
            ("<Alt>Left", "Back"),
            ("<Alt>Right", "Forward"),
            ("<Primary>r F5", "Reload or stop"),
            ("<Primary><Shift>r <Primary>F5", "Reload, ignoring cache"),
            ("<Primary>h", "History"),
        ],
    ),
    (
        "Page",
        &[
            ("<Primary>f", "Find in page"),
            ("<Primary>g F3", "Find next"),
            ("<Primary><Shift>g <Shift>F3", "Find previous"),
            ("<Primary>plus", "Zoom in"),
            ("<Primary>minus", "Zoom out"),
            ("<Primary>0", "Reset zoom"),
            ("<Primary>p", "Print"),
            ("<Primary><Shift>a", "Ask about this page"),
            ("F11", "Full screen"),
            ("<Primary>question", "Keyboard shortcuts"),
            ("F10", "Main menu"),
        ],
    ),
];

pub(crate) fn show_shortcuts_window(parent: &gtk::ApplicationWindow) {
    let mut groups = String::new();
    for (title, items) in SHORTCUTS {
        groups.push_str(&format!(
            r#"<child><object class="GtkShortcutsGroup"><property name="title">{}</property>"#,
            glib::markup_escape_text(title)
        ));
        for (accel, label) in *items {
            groups.push_str(&format!(
                r#"<child><object class="GtkShortcutsShortcut"><property name="accelerator">{}</property><property name="title">{}</property></object></child>"#,
                glib::markup_escape_text(accel),
                glib::markup_escape_text(label)
            ));
        }
        groups.push_str("</object></child>");
    }
    let ui = format!(
        r#"<interface><object class="GtkShortcutsWindow" id="window"><property name="modal">1</property><child><object class="GtkShortcutsSection"><property name="visible">1</property><property name="max-height">12</property>{groups}</object></child></object></interface>"#
    );
    let builder = gtk::Builder::from_string(&ui);
    let Some(window) = builder.object::<gtk::ShortcutsWindow>("window") else {
        eprintln!("Could not build the keyboard shortcuts window");
        return;
    };
    window.set_transient_for(Some(parent));
    window.show_all();
}
