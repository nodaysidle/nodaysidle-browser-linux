//! In-window find bar: construction, `FindController` queries, match-count
//! updates and narrow-width layout wiring.
//!
//! No `FindController` is ever stored: WebKit's controller keeps a raw,
//! non-owning pointer to its WebView, so every action looks up the controller
//! of the currently selected tab's view afresh (R-1).

use crate::tabs::{icon_button, TabManagerInner};
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Label};
use std::cell::RefCell;
use std::rc::Rc;
use webkit2gtk::{FindControllerExt, FindOptions, WebView, WebViewExt};

pub(crate) const FIND_MAX_MATCHES: u32 = 1_000;
const FIND_TYPING_PAUSE: std::time::Duration = std::time::Duration::from_millis(120);

#[derive(Clone)]
pub struct FindBar {
    pub(crate) bar: GtkBox,
    pub(crate) entry: gtk::Entry,
    pub(crate) status: Label,
    pub(crate) previous: Button,
    pub(crate) next: Button,
    pub(crate) close: Button,
}

pub(crate) fn build_find_bar() -> FindBar {
    let bar = GtkBox::new(gtk::Orientation::Horizontal, 6);
    bar.style_context().add_class("chrome");
    bar.style_context().add_class("find-bar");

    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some("Find in page"));
    entry.set_width_chars(10);
    entry.set_max_width_chars(32);
    entry.style_context().add_class("find-entry");
    let status = Label::new(None);
    status.set_ellipsize(gtk::pango::EllipsizeMode::End);
    status.set_width_chars(6);
    status.set_max_width_chars(24);
    status.style_context().add_class("find-status");
    let previous = icon_button("go-up-symbolic", "Previous match (Shift+Enter)");
    let next = icon_button("go-down-symbolic", "Next match (Enter)");
    let close = icon_button("window-close-symbolic", "Close find bar (Escape)");

    bar.pack_start(&entry, false, false, 0);
    bar.pack_start(&previous, false, false, 0);
    bar.pack_start(&next, false, false, 0);
    bar.pack_start(&status, false, false, 0);
    bar.pack_end(&close, false, false, 0);
    for child in bar.children() {
        child.show_all();
    }
    // Hidden until Ctrl+F; `window.show_all()` must not reveal it.
    bar.set_no_show_all(true);
    bar.hide();

    FindBar {
        bar,
        entry,
        status,
        previous,
        next,
        close,
    }
}

pub(crate) fn wire_find_bar(mgr: &Rc<RefCell<TabManagerInner>>) {
    let find_bar = mgr.borrow().find_bar.clone();

    // Each search walks the whole page twice (search + count), so wait for a
    // short pause in typing; clearing the field still takes effect at once.
    let mgr_changed = mgr.clone();
    let pending: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    find_bar.entry.connect_changed(move |entry| {
        if let Some(source) = pending.borrow_mut().take() {
            source.remove();
        }
        if entry.text().is_empty() {
            run_find(&mgr_changed);
            return;
        }
        let (mgr, fired) = (mgr_changed.clone(), pending.clone());
        let source = glib::timeout_add_local_once(FIND_TYPING_PAUSE, move || {
            // Forget the id first: removing a source that already ran panics.
            fired.borrow_mut().take();
            run_find(&mgr);
        });
        *pending.borrow_mut() = Some(source);
    });

    let mgr_activate = mgr.clone();
    find_bar
        .entry
        .connect_activate(move |_| find_step(&mgr_activate, false));

    let mgr_previous = mgr.clone();
    find_bar
        .previous
        .connect_clicked(move |_| find_step(&mgr_previous, true));

    let mgr_next = mgr.clone();
    find_bar
        .next
        .connect_clicked(move |_| find_step(&mgr_next, false));

    let mgr_close = mgr.clone();
    find_bar
        .close
        .connect_clicked(move |_| hide_find_bar(&mgr_close));

    let mgr_keys = mgr.clone();
    find_bar.entry.connect_key_press_event(move |_, event| {
        let key = event.keyval();
        if key == gdk::keys::constants::Escape {
            hide_find_bar(&mgr_keys);
            glib::Propagation::Stop
        } else if (key == gdk::keys::constants::Return || key == gdk::keys::constants::KP_Enter)
            && event.state().contains(gdk::ModifierType::SHIFT_MASK)
        {
            find_step(&mgr_keys, true);
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
}

/// The WebView currently shown in the selected tab, if any. Looked up on every
/// find action so no stale view or controller is ever used (R-1).
fn find_target(mgr: &Rc<RefCell<TabManagerInner>>) -> (FindBar, Option<WebView>) {
    let inner = mgr.borrow();
    (inner.find_bar.clone(), inner.selected_webview())
}

pub(crate) fn show_find_bar(mgr: &Rc<RefCell<TabManagerInner>>) {
    let find_bar = mgr.borrow().find_bar.clone();
    find_bar.bar.show();
    find_bar.entry.grab_focus();
    find_bar.entry.select_region(0, -1);
    run_find(mgr);
}

pub(crate) fn hide_find_bar(mgr: &Rc<RefCell<TabManagerInner>>) {
    let (find_bar, view) = find_target(mgr);
    if let Some(controller) = view.as_ref().and_then(|view| view.find_controller()) {
        controller.search_finish();
    }
    find_bar.bar.hide();
    set_find_status(&find_bar, FindStatus::Idle);
    if let Some(view) = view {
        view.grab_focus();
    }
}

/// Hides the find bar when a tab shows Home without clearing the query text.
/// Unlike `hide_find_bar`, it does not move focus (the caller decides that) and
/// it finishes the search on the tab's own view even though Home is displayed.
pub(crate) fn hide_find_bar_for_home(mgr: &Rc<RefCell<TabManagerInner>>, tab_id: u32) {
    let (find_bar, view) = {
        let inner = mgr.borrow();
        let view = inner
            .tabs
            .iter()
            .find(|tab| tab.id == tab_id)
            .and_then(|tab| tab.webview.clone());
        (inner.find_bar.clone(), view)
    };
    if !find_bar.bar.is_visible() {
        return;
    }
    if let Some(controller) = view.and_then(|view| view.find_controller()) {
        controller.search_finish();
    }
    find_bar.bar.hide();
    set_find_status(&find_bar, FindStatus::Idle);
}

pub(crate) fn run_find(mgr: &Rc<RefCell<TabManagerInner>>) {
    let (find_bar, view) = find_target(mgr);
    if !find_bar.bar.is_visible() {
        return;
    }
    let text = find_bar.entry.text();
    let Some(controller) = view.and_then(|view| view.find_controller()) else {
        set_find_status(
            &find_bar,
            if text.is_empty() {
                FindStatus::Idle
            } else {
                FindStatus::NoPage
            },
        );
        return;
    };
    if text.is_empty() {
        controller.search_finish();
        set_find_status(&find_bar, FindStatus::Idle);
    } else {
        set_find_status(&find_bar, FindStatus::Idle);
        controller.search(text.as_str(), find_options(), FIND_MAX_MATCHES);
        controller.count_matches(text.as_str(), find_options(), FIND_MAX_MATCHES);
    }
}

pub(crate) fn find_step(mgr: &Rc<RefCell<TabManagerInner>>, backwards: bool) {
    let (find_bar, view) = find_target(mgr);
    let text = find_bar.entry.text();
    let Some(controller) = view.and_then(|view| view.find_controller()) else {
        return;
    };
    if text.is_empty() {
        return;
    }
    if controller.search_text().as_deref() != Some(text.as_str()) {
        set_find_status(&find_bar, FindStatus::Idle);
        controller.search(text.as_str(), find_options(), FIND_MAX_MATCHES);
        controller.count_matches(text.as_str(), find_options(), FIND_MAX_MATCHES);
    } else if backwards {
        controller.search_previous();
    } else {
        controller.search_next();
    }
}

fn find_options() -> u32 {
    (FindOptions::CASE_INSENSITIVE | FindOptions::WRAP_AROUND).bits()
}

/// Per-view find feedback. The handlers live on the view's own controller, so
/// they disappear with the view; they only touch the bar when that view's tab
/// is still the selected one.
pub(crate) fn wire_find_feedback(mgr: &Rc<RefCell<TabManagerInner>>, tab_id: u32, view: &WebView) {
    let Some(controller) = view.find_controller() else {
        return;
    };
    let mgr_counted = Rc::downgrade(mgr);
    controller.connect_counted_matches(move |_, count| {
        if let Some(find_bar) = selected_find_bar(&mgr_counted, tab_id) {
            if count == 0 {
                set_find_status(&find_bar, FindStatus::NotFound);
            } else {
                set_find_status(&find_bar, FindStatus::Found(count));
            }
        }
    });
    let mgr_failed = Rc::downgrade(mgr);
    controller.connect_failed_to_find_text(move |_| {
        if let Some(find_bar) = selected_find_bar(&mgr_failed, tab_id) {
            set_find_status(&find_bar, FindStatus::NotFound);
        }
    });
}

fn selected_find_bar(
    mgr: &std::rc::Weak<RefCell<TabManagerInner>>,
    tab_id: u32,
) -> Option<FindBar> {
    let mgr = mgr.upgrade()?;
    let inner = mgr.try_borrow().ok()?;
    (inner.selected == Some(tab_id)).then(|| inner.find_bar.clone())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FindStatus {
    Idle,
    NoPage,
    NotFound,
    Found(u32),
}

pub(crate) fn find_status_text(status: FindStatus) -> String {
    match status {
        FindStatus::Idle => String::new(),
        FindStatus::NoPage => "No page to search".to_string(),
        FindStatus::NotFound => "No matches".to_string(),
        FindStatus::Found(1) => "1 match".to_string(),
        FindStatus::Found(count) if count > FIND_MAX_MATCHES => {
            format!("{FIND_MAX_MATCHES}+ matches")
        }
        FindStatus::Found(count) => format!("{count} matches"),
    }
}

pub(crate) fn set_find_status(find_bar: &FindBar, status: FindStatus) {
    find_bar.status.set_text(&find_status_text(status));
    let style = find_bar.entry.style_context();
    if matches!(status, FindStatus::NotFound | FindStatus::NoPage) {
        style.add_class("find-none");
    } else {
        style.remove_class("find-none");
    }
}
