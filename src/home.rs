use gtk::prelude::*;
use gtk::{Box as GtkBox, Entry, Label, Orientation};

/// Centered new-tab surface: "nodaysidle" + rounded search pill (DuckDuckGo via resolver).
pub fn build_home_surface() -> (GtkBox, Entry) {
    let page = GtkBox::new(Orientation::Vertical, 0);
    page.style_context().add_class("void");
    page.set_vexpand(true);

    let center = GtkBox::new(Orientation::Vertical, 18);
    center.set_valign(gtk::Align::Center);
    center.set_halign(gtk::Align::Center);

    let title = Label::new(None);
    title.set_markup("<span size='x-large' weight='semibold'>nodaysidle</span>");
    title.style_context().add_class("home-title");

    let pill = GtkBox::new(Orientation::Horizontal, 8);
    pill.set_width_request(520);
    pill.set_height_request(44);
    pill.style_context().add_class("home-search-pill");

    let search = Entry::new();
    search.set_hexpand(true);
    search.set_placeholder_text(Some("Search or type a URL…"));
    search.style_context().add_class("home-search-entry");

    pill.pack_start(&search, true, true, 0);

    center.pack_start(&title, false, false, 0);
    center.pack_start(&pill, false, false, 0);
    page.pack_start(&center, true, true, 0);

    (page, search)
}
