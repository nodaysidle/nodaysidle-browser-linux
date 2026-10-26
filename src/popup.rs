//! `window.open` sizing policy, transient pop-up windows, their keys and the
//! read-only address bar, plus the shared error/crash pages for every view.

use crate::navigation::title_for_page;
use crate::tabs::{shortcut_for, Shortcut, TabManager, TabManagerInner};
use gtk::prelude::*;
use gtk::{Box as GtkBox, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
use webkit2gtk::{WebView, WebViewExt};

/// A `window.open` whose features asked for a size. WebKitWindowProperties
/// alone cannot tell (observed on WebKitGTK 2.54): WebCore no longer sets the
/// bar visibility, a plain `window.open(url)` reports the opener window's size,
/// and a `target=_blank` link reports WebCore's 100x100 minimum. So links never
/// open pop-ups, and a window.open is a pop-up only when its geometry differs
/// from the opener window's size.
pub(crate) fn is_sized_popup(
    size: (i32, i32),
    opener_size: Option<(i32, i32)>,
    link_clicked: bool,
) -> bool {
    let (width, height) = size;
    !link_clicked && width > 0 && height > 0 && Some(size) != opener_size
}

/// Failed loads and crashed/killed web processes, shared by tabs and pop-up
/// windows (V-6). Cancelled loads and navigations that turned into downloads
/// are not failures the user needs to see.
pub(crate) fn wire_error_pages(webview: &WebView) {
    // Failed loads get a styled error page with Try again instead of a
    // blank view (X-21).
    webview.connect_load_failed(|view, _event, failing_uri, error| {
        if error.matches(webkit2gtk::NetworkError::Cancelled)
            || error.matches(webkit2gtk::PolicyError::FrameLoadInterruptedByPolicyChange)
            || error.matches(webkit2gtk::PluginError::WillHandleLoad)
        {
            return false;
        }
        let html = crate::error_page::load_failed_html(failing_uri, error.message());
        view.load_alternate_html(&html, failing_uri, None);
        true
    });

    webview.connect_load_failed_with_tls_errors(|view, failing_uri, _certificate, errors| {
        use gio::TlsCertificateFlags as F;
        let problems = [
            (F::UNKNOWN_CA, "is not from a trusted authority"),
            (F::BAD_IDENTITY, "is for a different site"),
            (F::NOT_ACTIVATED, "is not valid yet"),
            (F::EXPIRED, "has expired"),
            (F::REVOKED, "was revoked"),
            (F::INSECURE, "uses insecure cryptography"),
        ]
        .into_iter()
        .filter(|(flag, _)| errors.contains(*flag))
        .map(|(_, text)| text)
        .collect::<Vec<_>>();
        let html = crate::error_page::tls_failed_html(failing_uri, &problems);
        view.load_alternate_html(&html, failing_uri, None);
        true
    });

    // A crashed or killed web process used to leave a blank, dead tab
    // (X-27): explain it and offer Reload.
    webview.connect_web_process_terminated(|view, reason| {
        let exceeded_memory = match reason {
            webkit2gtk::WebProcessTerminationReason::TerminatedByApi => return,
            webkit2gtk::WebProcessTerminationReason::ExceededMemoryLimit => true,
            _ => false,
        };
        let uri = view.uri().unwrap_or_default().to_string();
        let html = crate::error_page::web_process_ended_html(&uri, exceeded_memory);
        let content_uri = if uri.is_empty() {
            "about:blank"
        } else {
            uri.as_str()
        };
        view.load_alternate_html(&html, content_uri, None);
    });
}

/// Keyboard for pop-up windows (V-6). Ctrl+W closes the window and
/// Ctrl+R / F5 reload, before the page sees the key, like in the main
/// window. Esc closes the pop-up only if the page did not handle it: this
/// handler runs after the default one, and WebKit hands keys the page did
/// not consume back to the window for that phase.
pub(crate) fn wire_popup_keys(window: &gtk::Window, view: &WebView, address: &gtk::Entry) {
    use gdk::keys::constants as key;
    use gdk::ModifierType as M;

    let modifiers = |event: &gdk::EventKey| {
        event.state() & (M::CONTROL_MASK | M::SHIFT_MASK | M::MOD1_MASK | M::SUPER_MASK)
    };
    let view_weak = view.downgrade();
    wire_popup_address_keys(window, address);
    window.connect_key_press_event(move |window, event| {
        let keyval = event.keyval().to_lower();
        let mods = modifiers(event);
        if mods == M::CONTROL_MASK && (keyval == key::w || keyval == key::F4) {
            window.close();
            return glib::Propagation::Stop;
        }
        let reload =
            (mods == M::CONTROL_MASK && keyval == key::r) || (mods.is_empty() && keyval == key::F5);
        if reload {
            if let Some(view) = view_weak.upgrade() {
                view.reload();
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.connect_local("key-press-event", true, move |values| {
        let handled = (|| {
            let window = values.first()?.get::<gtk::Window>().ok()?;
            let event = values.get(1)?.get::<gdk::Event>().ok()?;
            let event = event.downcast_ref::<gdk::EventKey>()?;
            if event.keyval() == key::Escape && modifiers(event).is_empty() {
                window.close();
                return Some(true);
            }
            None
        })();
        Some(handled.unwrap_or(false).to_value())
    });
}

pub(crate) fn wire_popup_address_keys(window: &gtk::Window, address: &gtk::Entry) {
    let address = address.downgrade();
    window.connect_key_press_event(move |_, event| {
        if shortcut_for(&event.keyval(), event.state()) == Some(Shortcut::FocusLocation) {
            if let Some(address) = address.upgrade() {
                address.grab_focus();
                address.select_region(0, -1);
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
}

/// Pop-up window for a sized `window.open`. It always shows the page's
/// address (read-only) so a sign-in pop-up cannot hide where it really is.
pub(crate) fn open_popup_window(
    mgr: &Rc<RefCell<TabManagerInner>>,
    view: &WebView,
    width: i32,
    height: i32,
) {
    let parent = mgr.borrow().window.clone();
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_transient_for(Some(&parent));
    window.set_destroy_with_parent(true);
    if let Some(app) = parent.application() {
        window.set_application(Some(&app));
    }
    window.set_default_size(width.clamp(200, 4_096), height.clamp(150, 4_096));
    window.set_title("Pop-up");
    window.style_context().add_class("browser-window");

    let frame = GtkBox::new(Orientation::Vertical, 0);
    frame.style_context().add_class("browser-frame");
    window.add(&frame);

    let root = GtkBox::new(Orientation::Vertical, 0);
    let address = gtk::Entry::new();
    address.set_editable(false);
    address.set_can_focus(true);
    address.set_tooltip_text(Some("Pop-up address (Ctrl+L or F6 to copy)"));
    address.style_context().add_class("url-bar");
    address.style_context().add_class("popup-address");
    let header = GtkBox::new(Orientation::Horizontal, 0);
    header.style_context().add_class("chrome");
    header.style_context().add_class("browser-toolbar");
    header.style_context().add_class("popup-chrome");
    header.pack_start(&address, true, true, 0);
    root.pack_start(&header, false, false, 0);
    view.set_vexpand(true);
    root.pack_start(view, true, true, 0);
    frame.pack_start(&root, true, true, 0);
    static NEXT_POPUP_FRAME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let popup_frame_id = format!(
        "nodaysidle-popup-frame-{}",
        NEXT_POPUP_FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    crate::theme::apply_browser_frame_insets(&frame, &popup_frame_id);
    crate::theme::wire_window_corner_insets(&window, &frame);

    let sync = {
        let window = window.downgrade();
        let address = address.downgrade();
        move |view: &WebView| {
            let uri = view.uri().unwrap_or_default().to_string();
            if let Some(address) = address.upgrade() {
                address.set_text(&crate::navigation::format_url_for_display(&uri));
            }
            if let Some(window) = window.upgrade() {
                window.set_title(&title_for_page(view.title().as_deref(), &uri));
            }
        }
    };
    sync(view);
    let sync_uri = sync.clone();
    view.connect_notify_local(Some("uri"), move |view, _| sync_uri(view));
    view.connect_notify_local(Some("title"), move |view, _| sync(view));

    crate::permissions::wire(view);
    wire_error_pages(view);
    wire_popup_keys(&window, view, &address);
    let mgr_create = mgr.clone();
    view.connect_create(move |parent, action| {
        Some(TabManager::create_related_view(&mgr_create, parent, action).upcast())
    });
    let window_weak = window.downgrade();
    view.connect_close(move |_| {
        let window_weak = window_weak.clone();
        glib::idle_add_local_once(move || {
            if let Some(window) = window_weak.upgrade() {
                window.close();
            }
        });
    });

    window.show_all();
    view.grab_focus();
}
