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
    clear_browsing_data(
        parent,
        &history,
        history_cb.is_active(),
        grants_cb.is_active(),
        web_context.website_data_manager().as_ref(),
        types,
    );
}

fn clear_browsing_data(
    parent: &gtk::ApplicationWindow,
    history: &Rc<RefCell<HistoryStore>>,
    clear_history: bool,
    clear_grants: bool,
    manager: Option<&WebsiteDataManager>,
    types: WebsiteDataTypes,
) {
    let mut failures = Vec::new();
    if clear_history && !history.borrow_mut().clear() {
        failures.push("Browsing history could not be cleared on disk.".to_string());
    }
    if clear_grants {
        crate::permissions::clear_session_grants();
    }
    let parent: glib::SendWeakRef<gtk::ApplicationWindow> = parent.downgrade().into();
    if types.is_empty() {
        report_clear_result(&parent, failures, Ok(()));
    } else if let Some(manager) = manager {
        manager.clear(
            types,
            glib::TimeSpan(0),
            None::<&gio::Cancellable>,
            move |result| {
                report_clear_result(&parent, failures, result);
            },
        );
    } else {
        failures.push("Cookies, site storage or cached files could not be cleared because the website data manager is unavailable.".to_string());
        report_clear_result(&parent, failures, Ok(()));
    }
}

fn report_clear_result(
    parent: &glib::WeakRef<gtk::ApplicationWindow>,
    mut failures: Vec<String>,
    result: Result<(), glib::Error>,
) -> Option<gtk::MessageDialog> {
    if let Err(err) = result {
        eprintln!("Could not clear website data: {err}");
        failures.push(
            "Selected cookies, site storage or cached files could not be cleared.".to_string(),
        );
    }
    if failures.is_empty() {
        return None;
    }
    eprintln!("{}", failures.join("\n"));
    // A destroyed GTK window can still have Rust references during completion.
    let parent = parent
        .upgrade()
        .filter(|parent| parent.is_visible() && !parent.in_destruction())?;
    let dialog = gtk::MessageDialog::new(
        Some(&parent),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        gtk::MessageType::Error,
        gtk::ButtonsType::Close,
        "Could not clear all selected browsing data",
    );
    dialog.set_secondary_text(Some(&failures.join("\n")));
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.show_all();
    Some(dialog)
}

#[cfg(test)]
pub(crate) fn verify_clear_data_feedback() {
    // `GtkWindow::close()` synthesises a delete event, so the window only
    // really hides once the main loop processes it.
    fn pump() {
        while gtk::events_pending() {
            gtk::main_iteration();
        }
    }

    let parent = gtk::ApplicationWindow::builder().build();
    parent.show_all();
    let weak = parent.downgrade();
    assert!(report_clear_result(&weak, Vec::new(), Ok(())).is_none());

    let dir = std::env::temp_dir().join(format!("nodaysidle-privacy-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let blocker = dir.join("profile");
    std::fs::write(&blocker, b"block profile writes").unwrap();
    let history = Rc::new(RefCell::new(HistoryStore::load(
        blocker.join("history.json"),
    )));
    history
        .borrow_mut()
        .record("https://example.com/".into(), "Example".into());
    clear_browsing_data(
        &parent,
        &history,
        true,
        false,
        None,
        WebsiteDataTypes::COOKIES,
    );
    let dialog = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|window| window.downcast::<gtk::MessageDialog>().ok())
        .find(|dialog| dialog.transient_for().as_ref() == Some(parent.upcast_ref()))
        .expect("history and missing-manager failures must produce visible feedback");
    let message = dialog.secondary_text().unwrap();
    assert!(message.contains("Browsing history"));
    assert!(message.contains("manager is unavailable"));
    assert!(dialog.is_visible());
    dialog.response(gtk::ResponseType::Close);
    pump();
    assert!(!dialog.is_visible());

    // History succeeds even when website-data deletion fails later.
    std::fs::remove_file(blocker).unwrap();
    assert!(history.borrow_mut().clear());
    assert!(HistoryStore::load(dir.join("profile/history.json"))
        .entries()
        .is_empty());
    let error = || glib::Error::new(gio::IOErrorEnum::Failed, "forced website-data failure");
    let dialog = report_clear_result(&weak, Vec::new(), Err(error())).unwrap();
    assert!(!dialog
        .secondary_text()
        .unwrap()
        .contains("Browsing history"));
    assert!(dialog
        .secondary_text()
        .unwrap()
        .contains("could not be cleared"));
    assert!(dialog.is_visible());
    dialog.response(gtk::ResponseType::Close);
    let dialog = report_clear_result(
        &weak,
        vec!["Browsing history could not be cleared on disk.".into()],
        Err(error()),
    )
    .unwrap();
    assert!(dialog
        .secondary_text()
        .unwrap()
        .contains("Browsing history"));
    assert!(dialog
        .secondary_text()
        .unwrap()
        .contains("Selected cookies"));

    // Model a completion arriving after destroy, while Rust still owns the parent.
    unsafe { parent.destroy() };
    pump();
    assert!(!dialog.is_visible());
    assert!(report_clear_result(&weak, Vec::new(), Err(error())).is_none());
    drop(parent);
    assert!(report_clear_result(&weak, vec!["failure".into()], Ok(())).is_none());
    std::fs::remove_dir_all(dir).unwrap();
}
