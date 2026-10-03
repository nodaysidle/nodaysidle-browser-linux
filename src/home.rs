use glib::clone;
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
    center.set_margin_start(16);
    center.set_margin_end(16);

    let title = Label::new(None);
    title.set_markup("<span size='x-large' weight='semibold'>nodaysidle</span>");
    title.style_context().add_class("home-title");

    // No fixed width: the pill is about 520 px wide when there is room and
    // shrinks with the window instead of setting its minimum width (X-24).
    let pill = GtkBox::new(Orientation::Horizontal, 8);
    pill.set_height_request(44);
    pill.style_context().add_class("home-search-pill");

    let search = Entry::new();
    search.set_hexpand(true);
    search.set_width_chars(12);
    search.set_max_width_chars(52);
    // Distinct from the address bar above it, and the pill lights up while it
    // has focus, so it is clear which of the two fields is active (I-4).
    search.set_placeholder_text(Some("Search DuckDuckGo or type a URL"));
    search.style_context().add_class("home-search-entry");
    let proceed = glib::Propagation::Proceed;
    search.connect_focus_in_event(clone!(@weak pill => @default-return proceed, move |_, _| {
        pill.style_context().add_class("focused");
        glib::Propagation::Proceed
    }));
    search.connect_focus_out_event(clone!(@weak pill => @default-return proceed, move |_, _| {
        pill.style_context().remove_class("focused");
        glib::Propagation::Proceed
    }));

    pill.pack_start(&search, true, true, 0);

    center.pack_start(&title, false, false, 0);
    center.pack_start(&pill, false, false, 0);
    page.pack_start(&center, true, true, 0);

    (page, search)
}
