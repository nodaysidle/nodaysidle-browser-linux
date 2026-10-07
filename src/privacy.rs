use crate::history::HistoryStore;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use webkit2gtk::{
    WebContext, WebContextExt, WebsiteDataManager, WebsiteDataManagerExtManual, WebsiteDataTypes,
};

pub fn show_clear_data_dialog(
    parent: &gtk::ApplicationWindow,
    history: Rc<RefCell<HistoryStore>>,
    web_context: &WebContext,
) {
    let dialog = gtk::Dialog::with_buttons(
        Some("Clear browsing data"),
        Some(parent),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        &[
            ("_Cancel", gtk::ResponseType::Cancel),
            ("_Clear", gtk::ResponseType::Accept),
        ],
    );
    dialog.set_default_response(gtk::ResponseType::Cancel);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let intro = gtk::Label::new(None);
    intro.set_line_wrap(true);
    intro.set_max_width_chars(52);
    intro.set_xalign(0.0);
    intro.set_text(
        "Choose what to remove from this device. This cannot be undone for the selected items.",
    );
    content.pack_start(&intro, false, false, 0);

    let history_cb = gtk::CheckButton::with_label("Browsing history");
    history_cb.set_active(true);
    let site_cb = gtk::CheckButton::with_label("Cookies and site storage");
    site_cb.set_active(true);
    let cache_cb = gtk::CheckButton::with_label("Cached files");
    cache_cb.set_active(false);
    let grants_cb = gtk::CheckButton::with_label("Remembered permission denials (this session)");
    grants_cb.set_active(true);

    for widget in [&history_cb, &site_cb, &cache_cb, &grants_cb] {
        content.pack_start(widget, false, false, 0);
    }
    dialog.content_area().pack_start(&content, true, true, 0);
    dialog.show_all();

    if dialog.run() != gtk::ResponseType::Accept {
        dialog.close();
        return;
    }
    dialog.close();

    if history_cb.is_active() && !history.borrow_mut().clear() {
        eprintln!("Could not clear browsing history on disk");
    }
    if grants_cb.is_active() {
        crate::permissions::clear_session_grants();
    }
    if site_cb.is_active() || cache_cb.is_active() {
        let Some(manager) = web_context.website_data_manager() else {
            eprintln!("Could not access the WebKit data manager to clear site data");
            return;
        };
        let mut types = WebsiteDataTypes::empty();
        if site_cb.is_active() {
            types |= WebsiteDataTypes::COOKIES
                | WebsiteDataTypes::LOCAL_STORAGE
                | WebsiteDataTypes::SESSION_STORAGE
                | WebsiteDataTypes::INDEXEDDB_DATABASES
                | WebsiteDataTypes::WEBSQL_DATABASES
                | WebsiteDataTypes::OFFLINE_APPLICATION_CACHE
                | WebsiteDataTypes::HSTS_CACHE
                | WebsiteDataTypes::ITP
                | WebsiteDataTypes::SERVICE_WORKER_REGISTRATIONS;
        }
        if cache_cb.is_active() {
            types |= WebsiteDataTypes::DISK_CACHE | WebsiteDataTypes::MEMORY_CACHE;
        }
        if types.is_empty() {
            return;
        }
        clear_website_data(&manager, types);
    }
}

fn clear_website_data(manager: &WebsiteDataManager, types: WebsiteDataTypes) {
    manager.clear(
        types,
        glib::TimeSpan(0),
        None::<&gio::Cancellable>,
        |result| {
            if let Err(err) = result {
                eprintln!("Could not clear website data: {err}");
            }
        },
    );
}
