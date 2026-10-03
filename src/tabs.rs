use crate::history::HistoryStore;
use crate::home::build_home_surface;
use crate::navigation::{resolve, title_for_page, SearchEngine};
use glib::clone;
use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Button, EventBox, Image, Label, ListBox, ListBoxRow, Orientation, Popover,
    ScrolledWindow, Separator, Stack,
};
use std::cell::RefCell;
use std::rc::Rc;
use webkit2gtk::{
    FindControllerExt, FindOptions, LoadEvent, SettingsExt as WebSettingsExt, WebView, WebViewExt,
    WindowPropertiesExt,
};

const TAB_BAR_HEIGHT: i32 = 36;
const TOOLBAR_HEIGHT: i32 = 40;

pub struct TabChrome {
    pub stack: Stack,
    pub tab_bar: GtkBox,
    pub tab_strip: GtkBox,
    pub tab_scroll: ScrolledWindow,
    pub tab_separator: Separator,
    pub toolbar: GtkBox,
    pub toolbar_separator: Separator,
    pub url_entry: gtk::Entry,
    pub back_btn: Button,
    pub forward_btn: Button,
    pub reload_btn: Button,
    pub home_btn: Button,
    pub find_bar: FindBar,
}

/// In-window find bar. It never stores a `FindController`: WebKit's controller
/// keeps a raw, non-owning pointer to its WebView, so every action looks up the
/// controller of the currently selected tab's view afresh (R-1).
#[derive(Clone)]
pub struct FindBar {
    bar: GtkBox,
    entry: gtk::Entry,
    status: Label,
    previous: Button,
    next: Button,
    close: Button,
}

const FIND_MAX_MATCHES: u32 = 1_000;

enum TabOpen {
    Home,
    Url(String),
}

struct TabEntry {
    id: u32,
    page_stack: Stack,
    /// Created on first navigation — home tabs stay webview-free until then.
    webview: Option<WebView>,
    home_search: gtk::Entry,
    pill: GtkBox,
    title_label: Label,
    close_btn: Button,
}

#[derive(Clone)]
pub struct TabManager {
    inner: Rc<RefCell<TabManagerInner>>,
}

struct TabManagerInner {
    stack: Stack,
    tab_bar: GtkBox,
    tab_strip: GtkBox,
    tab_scroll: ScrolledWindow,
    tab_separator: Separator,
    toolbar: GtkBox,
    toolbar_separator: Separator,
    tabs: Vec<TabEntry>,
    selected: Option<u32>,
    next_id: u32,
    url_entry: gtk::Entry,
    web_context: webkit2gtk::WebContext,
    history: Rc<RefCell<HistoryStore>>,
    search_engine: SearchEngine,
    back_btn: Button,
    forward_btn: Button,
    reload_btn: Button,
    home_btn: Button,
    find_bar: FindBar,
    window: gtk::ApplicationWindow,
    /// Tab whose page is in element (e.g. video) fullscreen, if any.
    fullscreen_owner: Option<u32>,
    /// Window fullscreen requested by the user with F11.
    user_fullscreen: bool,
    /// Pill waiting to be scrolled into view. Kept outside the RefCell'd state
    /// so size-allocate and adjustment handlers never borrow the manager.
    reveal: Rc<RefCell<Option<GtkBox>>>,
}

impl TabManager {
    pub fn new(
        window: &gtk::ApplicationWindow,
        chrome: TabChrome,
        web_context: webkit2gtk::WebContext,
        history: Rc<RefCell<HistoryStore>>,
    ) -> Self {
        let inner = TabManagerInner {
            stack: chrome.stack,
            tab_bar: chrome.tab_bar,
            tab_strip: chrome.tab_strip,
            tab_scroll: chrome.tab_scroll,
            tab_separator: chrome.tab_separator,
            toolbar: chrome.toolbar,
            toolbar_separator: chrome.toolbar_separator,
            tabs: Vec::new(),
            selected: None,
            next_id: 1,
            url_entry: chrome.url_entry,
            web_context,
            history,
            search_engine: SearchEngine::DuckDuckGo,
            back_btn: chrome.back_btn,
            forward_btn: chrome.forward_btn,
            reload_btn: chrome.reload_btn,
            home_btn: chrome.home_btn,
            find_bar: chrome.find_bar,
            window: window.clone(),
            fullscreen_owner: None,
            user_fullscreen: false,
            reveal: Rc::new(RefCell::new(None)),
        };
        wire_tab_reveal(&inner.tab_scroll, &inner.tab_strip, &inner.reveal);
        Self {
            inner: Rc::new(RefCell::new(inner)),
        }
    }

    pub fn clone_handle(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    pub fn open_initial_home(&self) {
        TabManager::open_tab(&self.inner, TabOpen::Home, true);
    }

    pub fn wire_toolbar(&self, new_tab_btn: &Button) {
        let mgr = self.inner.clone();
        new_tab_btn.connect_clicked(clone!(@strong mgr => move |_| {
            TabManager::open_tab(&mgr, TabOpen::Home, true);
        }));

        let mgr = self.inner.clone();
        self.inner.borrow().home_btn.connect_clicked(clone!(@strong mgr => move |_| {
            TabManager::navigate_home_for_selected(&mgr);
        }));

        let mgr = self.inner.clone();
        self.inner.borrow().back_btn.connect_clicked(clone!(@strong mgr => move |_| {
            let view = mgr.borrow().selected_webview();
            if let Some(view) = view {
                if view.can_go_back() {
                    view.go_back();
                }
            }
        }));

        let mgr = self.inner.clone();
        self.inner.borrow().forward_btn.connect_clicked(clone!(@strong mgr => move |_| {
            let view = mgr.borrow().selected_webview();
            if let Some(view) = view {
                if view.can_go_forward() {
                    view.go_forward();
                }
            }
        }));

        let mgr = self.inner.clone();
        self.inner.borrow().reload_btn.connect_clicked(clone!(@strong mgr => move |_| {
            if mgr.borrow().is_selected_on_home() {
                return;
            }
            let view = mgr.borrow().selected_webview();
            if let Some(view) = view {
                view.reload();
            }
        }));

        let mgr = self.inner.clone();
        self.inner.borrow().url_entry.connect_activate(clone!(@strong mgr => move |entry| {
            let text = entry.text().to_string();
            TabManager::navigate_selected(&mgr, &text);
        }));

        self.wire_find_bar();
    }

    fn wire_find_bar(&self) {
        let find_bar = self.inner.borrow().find_bar.clone();

        let mgr = self.inner.clone();
        find_bar.entry.connect_changed(move |_| run_find(&mgr));

        let mgr = self.inner.clone();
        find_bar
            .entry
            .connect_activate(move |_| find_step(&mgr, false));

        let mgr = self.inner.clone();
        find_bar.previous.connect_clicked(move |_| find_step(&mgr, true));

        let mgr = self.inner.clone();
        find_bar.next.connect_clicked(move |_| find_step(&mgr, false));

        let mgr = self.inner.clone();
        find_bar.close.connect_clicked(move |_| hide_find_bar(&mgr));

        let mgr = self.inner.clone();
        find_bar.entry.connect_key_press_event(move |_, event| {
            let key = event.keyval();
            if key == gdk::keys::constants::Escape {
                hide_find_bar(&mgr);
                glib::Propagation::Stop
            } else if (key == gdk::keys::constants::Return
                || key == gdk::keys::constants::KP_Enter)
                && event.state().contains(gdk::ModifierType::SHIFT_MASK)
            {
                find_step(&mgr, true);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }

    pub fn wire_keyboard(&self, window: &gtk::ApplicationWindow) {
        // Shortcuts are handled in the window's own key-press-event, which runs
        // before GTK's accelerators, focus-widget propagation and the WebView.
        // GtkAccelGroup could not carry Ctrl+Tab at all (Tab and ISO_Left_Tab
        // are not valid accelerator keys), and Shift+Tab arrives as
        // ISO_Left_Tab (N-3, R-5).
        let mgr = self.inner.clone();
        window.connect_key_press_event(move |_, event| {
            match shortcut_for(&event.keyval(), event.state()) {
                Some(shortcut) => {
                    run_shortcut(&mgr, shortcut);
                    glib::Propagation::Stop
                }
                None => glib::Propagation::Proceed,
            }
        });

        // The window manager can leave fullscreen on its own (e.g. a compositor
        // keybinding); keep the F11 state and the chrome in sync with it.
        let mgr = self.inner.clone();
        window.connect_window_state_event(move |_, event| {
            if event
                .changed_mask()
                .contains(gdk::WindowState::FULLSCREEN)
                && !event
                    .new_window_state()
                    .contains(gdk::WindowState::FULLSCREEN)
            {
                let restore = {
                    let Ok(mut inner) = mgr.try_borrow_mut() else {
                        return glib::Propagation::Proceed;
                    };
                    if inner.fullscreen_owner.is_none() && inner.user_fullscreen {
                        inner.user_fullscreen = false;
                        true
                    } else {
                        false
                    }
                };
                if restore {
                    set_browser_chrome_visible(&mgr, true);
                }
            }
            glib::Propagation::Proceed
        });
    }

    fn open_tab(mgr: &Rc<RefCell<TabManagerInner>>, open: TabOpen, select: bool) -> u32 {
        let (stack, tab_strip, next_id) = {
            let mut inner = mgr.borrow_mut();
            let id = inner.next_id;
            inner.next_id += 1;
            (inner.stack.clone(), inner.tab_strip.clone(), id)
        };

        let (home_page, home_search) = build_home_surface();
        home_search.set_text("");

        let page_stack = Stack::new();
        page_stack.add_named(&home_page, "home");
        page_stack.set_visible_child_name("home");

        let title_label = Label::new(Some("New Tab"));
        title_label.set_xalign(0.0);
        title_label.set_width_chars(8);
        title_label.set_max_width_chars(18);
        title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title_label.style_context().add_class("tab-pill-label");

        let close_btn = Button::new();
        let close_icon = Image::from_icon_name(Some("window-close-symbolic"), gtk::IconSize::Menu);
        close_btn.set_image(Some(&close_icon));
        close_btn.style_context().add_class("tab-close");
        close_btn.set_relief(gtk::ReliefStyle::None);
        // Ctrl+W closes the focused tab; the close button stays out of the Tab
        // order and never keeps focus after a click.
        close_btn.set_can_focus(false);
        close_btn.set_focus_on_click(false);

        let title_hit = EventBox::new();
        title_hit.add(&title_label);
        title_hit.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
        title_hit.set_can_focus(true);
        // Selecting a tab with the mouse moves focus into its page, so a click
        // must not leave the keyboard focus tint on the pill (R-6).
        title_hit.set_focus_on_click(false);
        title_hit.style_context().add_class("tab-focusable");

        let pill = GtkBox::new(Orientation::Horizontal, 0);
        pill.style_context().add_class("tab-pill");
        pill.pack_start(&title_hit, true, true, 0);
        pill.pack_start(&close_btn, false, false, 0);

        let tab_id = next_id;
        let name = tab_id.to_string();
        stack.add_named(&page_stack, &name);
        page_stack.show_all();
        tab_strip.pack_start(&pill, false, false, 0);
        pill.show_all();

        let mgr_weak = mgr.clone();
        title_hit.connect_button_press_event(clone!(@strong mgr_weak => move |_, event| {
            if event.button() == 1 {
                TabManager::select_tab_id(&mgr_weak, tab_id);
            }
            glib::Propagation::Proceed
        }));

        let mgr_key = mgr.clone();
        title_hit.connect_key_press_event(move |_, event| {
            let key = event.keyval();
            if key == gdk::keys::constants::Return || key == gdk::keys::constants::space {
                TabManager::select_tab_id(&mgr_key, tab_id);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });

        close_btn.connect_clicked(clone!(@strong mgr_weak => move |_| {
            TabManager::close_tab(&mgr_weak, tab_id);
        }));

        let mgr_nav = mgr.clone();
        home_search.connect_activate(clone!(@strong mgr_nav => move |entry| {
            let q = entry.text().to_string();
            TabManager::navigate_tab(&mgr_nav, tab_id, &q);
        }));

        mgr.borrow_mut().tabs.push(TabEntry {
            id: tab_id,
            page_stack,
            webview: None,
            home_search,
            pill,
            title_label,
            close_btn,
        });

        if select {
            mgr.borrow().url_entry.set_text("");
            TabManager::select_tab_id(mgr, tab_id);
        }
        match open {
            TabOpen::Home => {}
            TabOpen::Url(uri) => TabManager::load_uri_tab(mgr, tab_id, &uri),
        }
        tab_id
    }

    fn ensure_webview(mgr: &Rc<RefCell<TabManagerInner>>, tab_id: u32) -> Option<WebView> {
        let existing = {
            let inner = mgr.borrow();
            inner.tabs.iter().find(|tab| tab.id == tab_id)?.webview.clone()
        };
        if existing.is_some() {
            return existing;
        }
        TabManager::create_webview(mgr, tab_id)
    }

    fn create_webview(mgr: &Rc<RefCell<TabManagerInner>>, tab_id: u32) -> Option<WebView> {
        let web_context = {
            let inner = mgr.borrow();
            let tab = inner.tabs.iter().find(|tab| tab.id == tab_id)?;
            if let Some(view) = &tab.webview {
                return Some(view.clone());
            }
            inner.web_context.clone()
        };
        let webview = WebView::with_context(&web_context);
        configure_view(&webview);
        TabManager::attach_webview(mgr, tab_id, &webview)?;
        Some(webview)
    }

    /// Wires a WebView into a tab and shows it in that tab's page stack.
    fn attach_webview(
        mgr: &Rc<RefCell<TabManagerInner>>,
        tab_id: u32,
        webview: &WebView,
    ) -> Option<()> {
        let (history, page_stack) = {
            let inner = mgr.borrow();
            let tab = inner.tabs.iter().find(|tab| tab.id == tab_id)?;
            if tab.webview.is_some() {
                return None;
            }
            (inner.history.clone(), tab.page_stack.clone())
        };
        let webview = webview.clone();
        crate::permissions::wire(&webview);
        wire_find_feedback(mgr, tab_id, &webview);

        let mgr_load = mgr.clone();

        webview.connect_load_changed(clone!(@strong mgr_load, @strong history => move |view, ev| {
            // TLS information is known from Committed on.
            sync_page_status(&mgr_load, tab_id, view);
            if ev != LoadEvent::Finished {
                return;
            }
            let uri = view.uri().unwrap_or_default();
            if uri.is_empty() || uri == "about:blank" {
                return;
            }
            let title = title_for_page(view.title().as_deref(), &uri);
            let schedule_save = history.borrow_mut().record(uri.to_string(), title.clone());
            if schedule_save {
                // Batch history writes instead of rewriting the file on every
                // load (X-17); the app also flushes on shutdown.
                let history = history.clone();
                glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                    history.borrow_mut().flush();
                });
            }
            sync_view_chrome(&mgr_load, tab_id, view, &title, &uri);
        }));

        let mgr_uri = mgr.clone();
        webview.connect_notify_local(Some("uri"), move |view, _| {
            let uri = view.uri().unwrap_or_default().to_string();
            if uri.is_empty() {
                return;
            }
            let title = title_for_page(view.title().as_deref(), &uri);
            sync_view_chrome(&mgr_uri, tab_id, view, &title, &uri);
            sync_page_status(&mgr_uri, tab_id, view);
        });

        let mgr_title = mgr.clone();
        webview.connect_notify_local(Some("title"), move |view, _| {
            let uri = view.uri().unwrap_or_default().to_string();
            if uri.is_empty() {
                return;
            }
            let title = title_for_page(view.title().as_deref(), &uri);
            sync_view_chrome(&mgr_title, tab_id, view, &title, &uri);
        });

        // Element fullscreen (R-7): only hide or show our chrome and return
        // FALSE, so WebKit's default handler fullscreens the toplevel itself.
        // That handler copes with a window that is already fullscreen (F11),
        // which returning TRUE used to bypass.
        let mgr_enter = mgr.clone();
        webview.connect_enter_fullscreen(move |_| {
            {
                let Ok(mut inner) = mgr_enter.try_borrow_mut() else {
                    return false;
                };
                inner.fullscreen_owner = Some(tab_id);
            }
            set_browser_chrome_visible(&mgr_enter, false);
            false
        });

        let mgr_leave = mgr.clone();
        webview.connect_leave_fullscreen(move |_| {
            let show_chrome = {
                let Ok(mut inner) = mgr_leave.try_borrow_mut() else {
                    return false;
                };
                if inner.fullscreen_owner != Some(tab_id) {
                    return false;
                }
                inner.fullscreen_owner = None;
                !inner.user_fullscreen
            };
            if show_chrome {
                set_browser_chrome_visible(&mgr_leave, true);
            }
            false
        });

        let mgr_create = mgr.clone();
        webview.connect_create(move |parent, action| {
            Some(TabManager::create_related_view(&mgr_create, parent, action).upcast())
        });

        let mgr_close = mgr.clone();
        webview.connect_close(move |_| {
            // window.close() from the page: close the tab once WebKit has
            // finished emitting the signal.
            let mgr_close = mgr_close.clone();
            glib::idle_add_local_once(move || TabManager::close_tab(&mgr_close, tab_id));
        });

        {
            let mut inner = mgr.borrow_mut();
            let tab = inner.tabs.iter_mut().find(|tab| tab.id == tab_id)?;
            tab.webview = Some(webview.clone());
            inner.refresh_close_buttons();
        }
        page_stack.add_named(&webview, "web");
        webview.show_all();
        page_stack.set_visible_child_name("web");
        Some(())
    }

    /// `create` handler: returns a related view (required for window.opener
    /// and postMessage) and decides where it goes only on `ready-to-show`, as
    /// the WebKitGTK docs ask, because WebKitWindowProperties (the requested
    /// size) are known by then. Sized pop-ups (window.open with width and
    /// height, e.g. OAuth sign-in) get their own window; everything else
    /// (target=_blank, plain window.open) opens a selected tab (X-3).
    fn create_related_view(
        mgr: &Rc<RefCell<TabManagerInner>>,
        parent: &WebView,
        action: &webkit2gtk::NavigationAction,
    ) -> WebView {
        let view = WebView::with_related_view(parent);
        configure_view(&view);
        let link_clicked = action.navigation_type() == webkit2gtk::NavigationType::LinkClicked;
        let opener_size = parent
            .toplevel()
            .and_then(|widget| widget.downcast::<gtk::Window>().ok())
            .map(|window| window.size());
        let mgr_show = Rc::downgrade(mgr);
        view.connect_ready_to_show(move |view| {
            let Some(mgr) = mgr_show.upgrade() else {
                return;
            };
            if view.parent().is_some() {
                return;
            }
            let popup_size = view.window_properties().and_then(|properties| {
                let geometry = properties.geometry();
                let size = (geometry.width(), geometry.height());
                is_sized_popup(size, opener_size, link_clicked).then_some(size)
            });
            match popup_size {
                Some((width, height)) => open_popup_window(&mgr, view, width, height),
                None => {
                    let tab_id = TabManager::open_tab(&mgr, TabOpen::Home, true);
                    if TabManager::attach_webview(&mgr, tab_id, view).is_some() {
                        TabManager::select_tab_id(&mgr, tab_id);
                        let uri = view.uri().unwrap_or_default().to_string();
                        if !uri.is_empty() {
                            let title = title_for_page(view.title().as_deref(), &uri);
                            sync_view_chrome(&mgr, tab_id, view, &title, &uri);
                        }
                    }
                }
            }
        });
        view
    }

    fn navigate_tab(mgr: &Rc<RefCell<TabManagerInner>>, tab_id: u32, raw: &str) {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return;
        }
        let Some(url) = resolve(trimmed, mgr.borrow().search_engine) else {
            return;
        };
        TabManager::load_uri_tab(mgr, tab_id, &url);
    }

    fn load_uri_tab(mgr: &Rc<RefCell<TabManagerInner>>, tab_id: u32, uri: &str) {
        let Some(webview) = TabManager::ensure_webview(mgr, tab_id) else {
            return;
        };
        let (page_stack, url_entry) = {
            let inner = mgr.borrow();
            let tab = inner.tabs.iter().find(|t| t.id == tab_id);
            if tab.is_none() {
                return;
            }
            let tab = tab.unwrap();
            (tab.page_stack.clone(), inner.url_entry.clone())
        };
        webview.load_uri(uri);
        webview.show();
        page_stack.set_visible_child_name("web");
        url_entry.set_text(uri);
        if mgr.borrow().selected == Some(tab_id) {
            webview.grab_focus();
            mgr.borrow().refresh_nav_buttons();
        }
    }

    fn navigate_selected(mgr: &Rc<RefCell<TabManagerInner>>, raw: &str) {
        let id = mgr.borrow().selected;
        if let Some(id) = id {
            TabManager::navigate_tab(mgr, id, raw);
        }
    }

    fn navigate_home_for_selected(mgr: &Rc<RefCell<TabManagerInner>>) {
        let selected = { mgr.borrow().selected };
        if let Some(tab_id) = selected {
            TabManager::load_uri_tab(mgr, tab_id, crate::START_PAGE);
        }
    }

    fn select_tab_id(mgr: &Rc<RefCell<TabManagerInner>>, id: u32) {
        // Clear find highlights on the tab we are leaving while it is still alive;
        // the find bar is retargeted to the new tab below.
        let (find_visible, previous_view) = {
            let inner = mgr.borrow();
            let previous_view = (inner.selected != Some(id))
                .then(|| inner.selected_tab().and_then(|tab| tab.webview.clone()))
                .flatten();
            (inner.find_bar.bar.is_visible(), previous_view)
        };
        if find_visible {
            if let Some(controller) = previous_view.and_then(|view| view.find_controller()) {
                controller.search_finish();
            }
        }

        end_element_fullscreen_unless(mgr, Some(id));

        let (stack, url_entry, tab_scroll, tab_strip, reveal, sync) = {
            let mut inner = mgr.borrow_mut();
            inner.selected = Some(id);
            inner.refresh_close_buttons();
            for tab in &inner.tabs {
                let selected = tab.id == id;
                tab.pill.style_context().remove_class("selected");
                tab.title_label
                    .style_context()
                    .remove_class("tab-pill-label-selected");
                if selected {
                    tab.pill.style_context().add_class("selected");
                    tab.title_label
                        .style_context()
                        .add_class("tab-pill-label-selected");
                }
            }
            let sync = inner
                .tabs
                .iter()
                .find(|t| t.id == id)
                .map(|t| {
                    (
                        t.pill.clone(),
                        t.page_stack.clone(),
                        t.webview.clone(),
                        t.home_search.clone(),
                        t.title_label.clone(),
                    )
                });
            (
                inner.stack.clone(),
                inner.url_entry.clone(),
                inner.tab_scroll.clone(),
                inner.tab_strip.clone(),
                inner.reveal.clone(),
                sync,
            )
        };

        stack.set_visible_child_name(&id.to_string());
        if let Some((pill, page_stack, webview, home_search, title_label)) = sync {
            request_tab_reveal(&tab_scroll, &tab_strip, &reveal, pill);
            let on_home = page_stack.visible_child_name().as_deref() == Some("home")
                || webview.is_none();
            if on_home {
                page_stack.set_visible_child_name("home");
                url_entry.set_text("");
                title_label.set_text("New Tab");
                apply_page_status(&url_entry, None);
                home_search.grab_focus();
            } else if let Some(view) = webview {
                page_stack.set_visible_child_name("web");
                let uri = view.uri().unwrap_or_default();
                url_entry.set_text(&uri);
                apply_page_status(&url_entry, Some(&view));
                view.grab_focus();
            }
        }
        mgr.borrow().refresh_nav_buttons();
        if find_visible {
            run_find(mgr);
        }
    }

    /// Closes a tab. Closing the last tab replaces it with a fresh Home tab
    /// rather than leaving the window without one (N-2); a lone Home tab that
    /// never loaded a page has nothing to close and stays as it is.
    fn close_tab(mgr: &Rc<RefCell<TabManagerInner>>, id: u32) {
        let last_tab = {
            let inner = mgr.borrow();
            let Some(tab) = inner.tabs.iter().find(|tab| tab.id == id) else {
                return;
            };
            last_tab_action(inner.tabs.len(), tab.webview.is_some())
        };
        match last_tab {
            LastTab::NotLast => {}
            LastTab::KeepPristineHome => {
                let home_search = {
                    let inner = mgr.borrow();
                    inner.tabs.iter().find(|tab| tab.id == id).map(|tab| tab.home_search.clone())
                };
                if let Some(home_search) = home_search {
                    home_search.grab_focus();
                }
                return;
            }
            LastTab::ReplaceWithHome => {
                TabManager::open_tab(mgr, TabOpen::Home, true);
            }
        }
        let owns_fullscreen = { mgr.borrow().fullscreen_owner == Some(id) };
        if owns_fullscreen {
            end_element_fullscreen_unless(mgr, None);
        }
        // Only take the entry out while borrowed. Removing widgets emits GTK
        // signals and dropping the last reference to a WebView runs WebKit
        // teardown, so both happen after the borrow ends (X-28).
        let (tab, stack, tab_strip, removed_selected, select_after) = {
            let mut inner = mgr.borrow_mut();
            let Some(idx) = inner.tabs.iter().position(|t| t.id == id) else {
                return;
            };
            let tab = inner.tabs.remove(idx);
            let was_selected = inner.selected == Some(id);
            let remaining_ids = inner.tabs.iter().map(|tab| tab.id).collect::<Vec<_>>();
            let selected_after = selection_after_close(&remaining_ids, inner.selected, id, idx);
            let select_after = if was_selected {
                inner.selected = None;
                selected_after
            } else {
                None
            };
            (
                tab,
                inner.stack.clone(),
                inner.tab_strip.clone(),
                was_selected,
                select_after,
            )
        };

        stack.remove(&tab.page_stack);
        tab_strip.remove(&tab.pill);
        mgr.borrow().refresh_close_buttons();
        let TabEntry { page_stack, .. } = tab;
        // Destroy the tab's page (and with it its WebView) explicitly instead
        // of relying on the last reference going away: signal closures, pop-up
        // openers or pending callbacks can keep a view alive, and a closed tab
        // must stop running its page (R-2). Deferred to an idle so no handler of
        // this view is still on the stack.
        glib::idle_add_local_once(move || {
            // SAFETY: the page stack is no longer in the widget tree or in
            // TabManager, nothing looks it or its WebView up again (every
            // handler resolves its tab by id, which is gone), and destroy()
            // only disposes the widgets; remaining references stay valid
            // GObjects.
            unsafe { page_stack.destroy() };
        });

        if let Some(next_id) = select_after {
            TabManager::select_tab_id(mgr, next_id);
        } else if removed_selected {
            mgr.borrow().refresh_nav_buttons();
        }
    }

    /// Raises the browser window (second launch, external open). GTK uses the
    /// startup-notification / activation token GApplication received from the
    /// launcher, so compositors with focus-stealing prevention accept it.
    /// Writes pending history to disk (called on application shutdown).
    pub fn flush_history(&self) {
        let history = { self.inner.borrow().history.clone() };
        history.borrow_mut().flush();
    }

    pub fn present(&self) {
        let window = { self.inner.borrow().window.clone() };
        window.present();
    }

    pub fn navigate_from_bar_public(&self, raw: &str) {
        TabManager::navigate_selected(&self.inner, raw);
    }

    pub fn open_external_uri(&self, uri: &str) {
        let selected_home = {
            let inner = self.inner.borrow();
            if inner.is_selected_on_home() {
                inner.selected
            } else {
                None
            }
        };
        if let Some(tab_id) = selected_home {
            TabManager::load_uri_tab(&self.inner, tab_id, uri);
            return;
        }
        TabManager::open_tab(&self.inner, TabOpen::Url(uri.to_string()), true);
    }

    pub fn wire_history(&self, history: Rc<RefCell<HistoryStore>>) {
        let history_btn = Button::new();
        let img = Image::from_icon_name(Some("document-open-recent-symbolic"), gtk::IconSize::Button);
        history_btn.set_image(Some(&img));
        history_btn.set_tooltip_text(Some("History"));
        history_btn.style_context().add_class("ghost-btn");
        history_btn.set_relief(gtk::ReliefStyle::None);
        history_btn.set_focus_on_click(false);

        if let Some(toolbar) = self
            .inner
            .borrow()
            .url_entry
            .parent()
            .and_then(|p| p.downcast::<GtkBox>().ok())
        {
            toolbar.pack_end(&history_btn, false, false, 0);
            history_btn.show();
        }

        let popover = Popover::new(Some(&history_btn));
        let box_ = GtkBox::new(Orientation::Vertical, 0);
        let title = Label::new(Some("Recent history"));
        title.set_margin_top(8);
        title.set_margin_start(12);
        title.set_margin_end(12);
        box_.pack_start(&title, false, false, 0);
        box_.pack_start(&Separator::new(Orientation::Horizontal), false, false, 0);

        let scroll = ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_min_content_width(420);
        scroll.set_min_content_height(320);
        let list = ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        scroll.add(&list);
        box_.pack_start(&scroll, true, true, 0);
        popover.add(&box_);

        let mgr = self.clone_handle();
        history_btn.connect_clicked(clone!(@weak popover, @weak list, @strong history, @strong mgr => move |_| {
            while let Some(row) = list.row_at_index(0) {
                list.remove(&row);
            }
            for entry in history.borrow().entries().iter() {
                list.add(&history_row(entry));
            }
            popover.show_all();
            popover.popup();
        }));

        list.connect_row_activated(clone!(@weak popover, @strong mgr => move |_, row| {
            let url = row.widget_name().to_string();
            if url.is_empty() {
                return;
            }
            popover.popdown();
            mgr.navigate_from_bar_public(&url);
        }));
    }
}

impl TabManagerInner {
    /// Every tab can be closed except a lone Home tab that never loaded a page.
    fn refresh_close_buttons(&self) {
        let only_tab_has_page = self.tabs.first().is_some_and(|tab| tab.webview.is_some());
        let closable = self.tabs.len() > 1 || only_tab_has_page;
        for tab in &self.tabs {
            tab.close_btn.set_visible(closable);
        }
    }

    fn selected_tab(&self) -> Option<&TabEntry> {
        let id = self.selected?;
        self.tabs.iter().find(|t| t.id == id)
    }

    fn selected_webview(&self) -> Option<WebView> {
        if self.is_selected_on_home() {
            return None;
        }
        self.selected_tab().and_then(|t| t.webview.clone())
    }

    fn is_selected_on_home(&self) -> bool {
        match self.selected_tab() {
            Some(tab) => {
                tab.webview.is_none()
                    || tab.page_stack.visible_child_name().as_deref() == Some("home")
            }
            None => false,
        }
    }

    fn refresh_nav_buttons(&self) {
        if self.is_selected_on_home() {
            self.back_btn.set_sensitive(false);
            self.forward_btn.set_sensitive(false);
            self.reload_btn.set_sensitive(false);
            return;
        }
        let view = self.selected_tab().and_then(|t| t.webview.clone());
        let can_back = view.as_ref().map(|v| v.can_go_back()).unwrap_or(false);
        let can_fwd = view.as_ref().map(|v| v.can_go_forward()).unwrap_or(false);
        self.back_btn.set_sensitive(can_back);
        self.forward_btn.set_sensitive(can_fwd);
        self.reload_btn.set_sensitive(true);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LastTab {
    NotLast,
    /// The only tab has loaded a page: open a fresh Home tab, then close it.
    ReplaceWithHome,
    /// The only tab is an untouched Home tab: nothing to do.
    KeepPristineHome,
}

fn last_tab_action(tab_count: usize, has_page: bool) -> LastTab {
    match (tab_count, has_page) {
        (0 | 1, true) => LastTab::ReplaceWithHome,
        (0 | 1, false) => LastTab::KeepPristineHome,
        _ => LastTab::NotLast,
    }
}

fn configure_view(webview: &WebView) {
    if let Some(settings) = WebViewExt::settings(webview) {
        settings.set_enable_javascript(true);
        settings.set_enable_html5_database(true);
        settings.set_enable_html5_local_storage(true);
    }
}

/// A `window.open` whose features asked for a size. WebKitWindowProperties
/// alone cannot tell (observed on WebKitGTK 2.54): WebCore no longer sets the
/// bar visibility, a plain `window.open(url)` reports the opener window's size,
/// and a `target=_blank` link reports WebCore's 100x100 minimum. So links never
/// open pop-ups, and a window.open is a pop-up only when its geometry differs
/// from the opener window's size.
fn is_sized_popup(size: (i32, i32), opener_size: Option<(i32, i32)>, link_clicked: bool) -> bool {
    let (width, height) = size;
    !link_clicked && width > 0 && height > 0 && Some(size) != opener_size
}

/// Pop-up window for a sized `window.open`. It always shows the page's
/// address (read-only) so a sign-in pop-up cannot hide where it really is.
fn open_popup_window(mgr: &Rc<RefCell<TabManagerInner>>, view: &WebView, width: i32, height: i32) {
    let parent = { mgr.borrow().window.clone() };
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_transient_for(Some(&parent));
    window.set_destroy_with_parent(true);
    if let Some(app) = parent.application() {
        window.set_application(Some(&app));
    }
    window.set_default_size(width.clamp(200, 4_096), height.clamp(150, 4_096));
    window.set_title("Pop-up");

    let root = GtkBox::new(Orientation::Vertical, 0);
    let address = gtk::Entry::new();
    address.set_editable(false);
    address.set_can_focus(false);
    address.style_context().add_class("url-bar");
    address.style_context().add_class("popup-address");
    let header = GtkBox::new(Orientation::Horizontal, 0);
    header.style_context().add_class("chrome");
    header.style_context().add_class("toolbar");
    header.pack_start(&address, true, true, 0);
    root.pack_start(&header, false, false, 0);
    view.set_vexpand(true);
    root.pack_start(view, true, true, 0);
    window.add(&root);

    let sync = {
        let window = window.downgrade();
        let address = address.downgrade();
        move |view: &WebView| {
            let uri = view.uri().unwrap_or_default().to_string();
            if let Some(address) = address.upgrade() {
                address.set_text(&uri);
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

fn sync_view_chrome(
    mgr: &Rc<RefCell<TabManagerInner>>,
    tab_id: u32,
    view: &WebView,
    title: &str,
    uri: &str,
) {
    let (selected, url_entry, title_label, back_btn, forward_btn, reload_btn) = {
        let inner = mgr.borrow();
        let Some(tab) = inner.tabs.iter().find(|tab| tab.id == tab_id) else {
            return;
        };
        (
            inner.selected == Some(tab_id),
            inner.url_entry.clone(),
            tab.title_label.clone(),
            inner.back_btn.clone(),
            inner.forward_btn.clone(),
            inner.reload_btn.clone(),
        )
    };

    title_label.set_text(&truncate(title));
    if !selected {
        return;
    }

    if let Some(uri) = url_bar_sync_value(url_entry.has_focus(), uri) {
        url_entry.set_text(uri);
    }
    back_btn.set_sensitive(view.can_go_back());
    forward_btn.set_sensitive(view.can_go_forward());
    reload_btn.set_sensitive(true);
}

/// Refreshes the address bar's connection indicator for `tab_id` if it is the
/// selected tab.
fn sync_page_status(mgr: &Rc<RefCell<TabManagerInner>>, tab_id: u32, view: &WebView) {
    let url_entry = {
        let Ok(inner) = mgr.try_borrow() else {
            return;
        };
        if inner.selected != Some(tab_id) {
            return;
        }
        inner.url_entry.clone()
    };
    apply_page_status(&url_entry, Some(view));
}

/// `view` is None for the built-in Home page.
fn apply_page_status(url_entry: &gtk::Entry, view: Option<&WebView>) {
    let security = view.map_or(Security::None, |view| {
        let uri = view.uri().unwrap_or_default();
        let tls_errors = view.tls_info().map(|(_, errors)| !errors.is_empty());
        security_state(&uri, tls_errors)
    });
    let (icon, tooltip, class) = match security {
        Security::Secure => (
            Some("channel-secure-symbolic"),
            Some("Secure connection (HTTPS)"),
            Some("secure"),
        ),
        Security::Insecure => (
            Some("channel-insecure-symbolic"),
            Some("Not secure: this page does not use a valid HTTPS connection"),
            Some("insecure"),
        ),
        Security::None => (None, None, None),
    };
    url_entry.set_icon_from_icon_name(gtk::EntryIconPosition::Primary, icon);
    url_entry.set_icon_tooltip_text(gtk::EntryIconPosition::Primary, tooltip);
    let style = url_entry.style_context();
    style.remove_class("secure");
    style.remove_class("insecure");
    if let Some(class) = class {
        style.add_class(class);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Security {
    /// HTTPS with a certificate WebKit accepted without errors.
    Secure,
    /// Plain HTTP to a remote host, or HTTPS with certificate errors.
    Insecure,
    /// Local, file:, about: and other pages: no indicator.
    None,
}

/// `tls_errors` is WebKit's TLS info for the page: None when it has none,
/// Some(true) when the certificate had errors (X-20).
fn security_state(uri: &str, tls_errors: Option<bool>) -> Security {
    let Ok(parsed) = url::Url::parse(uri) else {
        return Security::None;
    };
    match parsed.scheme() {
        "https" => match tls_errors {
            Some(false) => Security::Secure,
            Some(true) => Security::Insecure,
            // Not committed yet: say nothing rather than guess.
            None => Security::None,
        },
        "http" => {
            let local = match parsed.host() {
                Some(url::Host::Domain(host)) => host == "localhost" || host.ends_with(".localhost"),
                Some(url::Host::Ipv4(address)) => address.is_loopback(),
                Some(url::Host::Ipv6(address)) => address.is_loopback(),
                None => true,
            };
            if local {
                Security::None
            } else {
                Security::Insecure
            }
        }
        _ => Security::None,
    }
}

fn url_bar_sync_value(is_focused: bool, uri: &str) -> Option<&str> {
    (!is_focused).then_some(uri)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shortcut {
    NewTab,
    CloseTab,
    FocusLocation,
    NextTab,
    PreviousTab,
    /// Ctrl+1..Ctrl+8 select a tab by position; Ctrl+9 selects the last tab.
    SelectTab(usize),
    LastTab,
    Find,
    ToggleFullscreen,
    Reload,
    Back,
    Forward,
}

fn shortcut_for(keyval: &gdk::keys::Key, state: gdk::ModifierType) -> Option<Shortcut> {
    use gdk::keys::constants as key;
    use gdk::ModifierType as M;

    let mods = state & (M::CONTROL_MASK | M::SHIFT_MASK | M::MOD1_MASK | M::SUPER_MASK);
    let ctrl = mods == M::CONTROL_MASK;
    let ctrl_shift = mods == M::CONTROL_MASK | M::SHIFT_MASK;
    let alt = mods == M::MOD1_MASK;
    let none = mods.is_empty();
    let keyval = keyval.to_lower();

    if (ctrl && (keyval == key::Tab || keyval == key::Page_Down || keyval == key::KP_Page_Down))
        || (ctrl_shift && keyval == key::Page_Down)
    {
        return Some(Shortcut::NextTab);
    }
    if (ctrl_shift && (keyval == key::ISO_Left_Tab || keyval == key::Tab))
        || (ctrl && (keyval == key::ISO_Left_Tab || keyval == key::Page_Up || keyval == key::KP_Page_Up))
    {
        return Some(Shortcut::PreviousTab);
    }
    if ctrl {
        let digit = [
            key::_1, key::_2, key::_3, key::_4, key::_5, key::_6, key::_7, key::_8,
        ]
        .iter()
        .position(|candidate| *candidate == keyval);
        if let Some(index) = digit {
            return Some(Shortcut::SelectTab(index));
        }
        if keyval == key::_9 {
            return Some(Shortcut::LastTab);
        }
    }
    let shortcut = if ctrl && keyval == key::t {
        Shortcut::NewTab
    } else if ctrl && (keyval == key::w || keyval == key::F4) {
        Shortcut::CloseTab
    } else if (ctrl && keyval == key::l) || (alt && keyval == key::d) || (none && keyval == key::F6) {
        Shortcut::FocusLocation
    } else if ctrl && keyval == key::f {
        Shortcut::Find
    } else if none && keyval == key::F11 {
        Shortcut::ToggleFullscreen
    } else if (ctrl && keyval == key::r) || (none && keyval == key::F5) {
        Shortcut::Reload
    } else if alt && (keyval == key::Left || keyval == key::KP_Left) {
        Shortcut::Back
    } else if alt && (keyval == key::Right || keyval == key::KP_Right) {
        Shortcut::Forward
    } else {
        return None;
    };
    Some(shortcut)
}

fn run_shortcut(mgr: &Rc<RefCell<TabManagerInner>>, shortcut: Shortcut) {
    match shortcut {
        Shortcut::NewTab => {
            TabManager::open_tab(mgr, TabOpen::Home, true);
        }
        Shortcut::CloseTab => {
            let selected = { mgr.borrow().selected };
            if let Some(tab_id) = selected {
                TabManager::close_tab(mgr, tab_id);
            }
        }
        Shortcut::FocusLocation => {
            let url_entry = { mgr.borrow().url_entry.clone() };
            url_entry.grab_focus();
            url_entry.select_region(0, -1);
        }
        Shortcut::NextTab => cycle_selected_tab(mgr, false),
        Shortcut::PreviousTab => cycle_selected_tab(mgr, true),
        Shortcut::SelectTab(index) => {
            let tab_id = { mgr.borrow().tabs.get(index).map(|tab| tab.id) };
            if let Some(tab_id) = tab_id {
                TabManager::select_tab_id(mgr, tab_id);
            }
        }
        Shortcut::LastTab => {
            let tab_id = { mgr.borrow().tabs.last().map(|tab| tab.id) };
            if let Some(tab_id) = tab_id {
                TabManager::select_tab_id(mgr, tab_id);
            }
        }
        Shortcut::Find => show_find_bar(mgr),
        Shortcut::ToggleFullscreen => toggle_user_fullscreen(mgr),
        Shortcut::Reload => {
            let view = { mgr.borrow().selected_webview() };
            if let Some(view) = view {
                view.reload();
            }
        }
        Shortcut::Back => {
            let view = { mgr.borrow().selected_webview() };
            if let Some(view) = view.filter(|view| view.can_go_back()) {
                view.go_back();
            }
        }
        Shortcut::Forward => {
            let view = { mgr.borrow().selected_webview() };
            if let Some(view) = view.filter(|view| view.can_go_forward()) {
                view.go_forward();
            }
        }
    }
}

fn cycle_selected_tab(mgr: &Rc<RefCell<TabManagerInner>>, reverse: bool) {
    let next = {
        let inner = mgr.borrow();
        let ids = inner.tabs.iter().map(|tab| tab.id).collect::<Vec<_>>();
        next_tab_id(&ids, inner.selected, reverse)
    };
    if let Some(tab_id) = next {
        TabManager::select_tab_id(mgr, tab_id);
    }
}

fn next_tab_id(ids: &[u32], selected: Option<u32>, reverse: bool) -> Option<u32> {
    if ids.is_empty() {
        return None;
    }
    let current_index = selected.and_then(|id| ids.iter().position(|candidate| *candidate == id));
    let next_index = match (current_index, reverse) {
        (Some(index), true) => (index + ids.len() - 1) % ids.len(),
        (Some(index), false) => (index + 1) % ids.len(),
        (None, true) => ids.len() - 1,
        (None, false) => 0,
    };
    Some(ids[next_index])
}

/// The WebView currently shown in the selected tab, if any. Looked up on every
/// find action so no stale view or controller is ever used (R-1).
fn find_target(mgr: &Rc<RefCell<TabManagerInner>>) -> (FindBar, Option<WebView>) {
    let inner = mgr.borrow();
    (inner.find_bar.clone(), inner.selected_webview())
}

fn show_find_bar(mgr: &Rc<RefCell<TabManagerInner>>) {
    let find_bar = { mgr.borrow().find_bar.clone() };
    find_bar.bar.show();
    find_bar.entry.grab_focus();
    find_bar.entry.select_region(0, -1);
    run_find(mgr);
}

fn hide_find_bar(mgr: &Rc<RefCell<TabManagerInner>>) {
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

fn run_find(mgr: &Rc<RefCell<TabManagerInner>>) {
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
        controller.search(text.as_str(), find_options(), FIND_MAX_MATCHES);
    }
}

fn find_step(mgr: &Rc<RefCell<TabManagerInner>>, backwards: bool) {
    let (find_bar, view) = find_target(mgr);
    let text = find_bar.entry.text();
    let Some(controller) = view.and_then(|view| view.find_controller()) else {
        return;
    };
    if text.is_empty() {
        return;
    }
    if controller.search_text().as_deref() != Some(text.as_str()) {
        controller.search(text.as_str(), find_options(), FIND_MAX_MATCHES);
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
fn wire_find_feedback(mgr: &Rc<RefCell<TabManagerInner>>, tab_id: u32, view: &WebView) {
    let Some(controller) = view.find_controller() else {
        return;
    };
    let mgr_found = Rc::downgrade(mgr);
    controller.connect_found_text(move |_, count| {
        if let Some(find_bar) = selected_find_bar(&mgr_found, tab_id) {
            set_find_status(&find_bar, FindStatus::Found(count));
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
enum FindStatus {
    Idle,
    NoPage,
    NotFound,
    Found(u32),
}

fn find_status_text(status: FindStatus) -> String {
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

fn set_find_status(find_bar: &FindBar, status: FindStatus) {
    find_bar.status.set_text(&find_status_text(status));
    let style = find_bar.entry.style_context();
    if matches!(status, FindStatus::NotFound | FindStatus::NoPage) {
        style.add_class("find-none");
    } else {
        style.remove_class("find-none");
    }
}

fn toggle_user_fullscreen(mgr: &Rc<RefCell<TabManagerInner>>) {
    let (window, owner_view, window_is_fullscreen) = {
        let inner = mgr.borrow();
        let owner_view = inner.fullscreen_owner.and_then(|owner| {
            inner
                .tabs
                .iter()
                .find(|tab| tab.id == owner)
                .and_then(|tab| tab.webview.clone())
        });
        let window_is_fullscreen = inner
            .window
            .window()
            .map(|window| window.state().contains(gdk::WindowState::FULLSCREEN))
            .unwrap_or(false);
        (inner.window.clone(), owner_view, window_is_fullscreen)
    };
    if let Some(view) = owner_view {
        // F11 while a video is fullscreen leaves the video fullscreen first.
        exit_element_fullscreen(&view);
        return;
    }
    let user_fullscreen = !window_is_fullscreen;
    {
        mgr.borrow_mut().user_fullscreen = user_fullscreen;
    }
    set_browser_chrome_visible(mgr, !user_fullscreen);
    if user_fullscreen {
        window.fullscreen();
    } else {
        window.unfullscreen();
    }
}

/// Ends element fullscreen when the owning tab is closed or another tab is
/// selected, restoring the chrome and the window's F11 state (R-7).
fn end_element_fullscreen_unless(mgr: &Rc<RefCell<TabManagerInner>>, keep: Option<u32>) {
    let (owner_view, window, user_fullscreen) = {
        let mut inner = mgr.borrow_mut();
        let Some(owner) = inner.fullscreen_owner else {
            return;
        };
        if Some(owner) == keep {
            return;
        }
        inner.fullscreen_owner = None;
        let owner_view = inner
            .tabs
            .iter()
            .find(|tab| tab.id == owner)
            .and_then(|tab| tab.webview.clone());
        (owner_view, inner.window.clone(), inner.user_fullscreen)
    };
    if let Some(view) = owner_view {
        exit_element_fullscreen(&view);
    }
    if !user_fullscreen {
        window.unfullscreen();
    }
    set_browser_chrome_visible(mgr, !user_fullscreen);
}

fn exit_element_fullscreen(view: &WebView) {
    view.evaluate_javascript(
        "if (document.fullscreenElement) { document.exitFullscreen(); }",
        None,
        None,
        None::<&gio::Cancellable>,
        |_| {},
    );
}

fn set_browser_chrome_visible(mgr: &Rc<RefCell<TabManagerInner>>, visible: bool) {
    let (tab_bar, tab_separator, toolbar, toolbar_separator) = {
        let inner = mgr.borrow();
        (
            inner.tab_bar.clone(),
            inner.tab_separator.clone(),
            inner.toolbar.clone(),
            inner.toolbar_separator.clone(),
        )
    };
    tab_bar.set_visible(visible);
    tab_separator.set_visible(visible);
    toolbar.set_visible(visible);
    toolbar_separator.set_visible(visible);
}

fn selection_after_close(
    remaining_ids: &[u32],
    selected: Option<u32>,
    closed_id: u32,
    closed_index: usize,
) -> Option<u32> {
    if selected == Some(closed_id) {
        remaining_ids
            .get(closed_index.min(remaining_ids.len().saturating_sub(1)))
            .copied()
    } else {
        selected.filter(|id| remaining_ids.contains(id))
    }
}

/// Scrolls the selected pill into view once GTK has really laid it out (R-3).
/// A new pill still has GTK's placeholder allocation (x = -1, width = 1) right
/// after it is packed, and the scrolled window's adjustment learns the new
/// strip width only in a later size-allocate, so a one-shot idle could scroll
/// to the start. The request stays pending and is retried on every strip
/// allocation and adjustment change until the pill is actually visible.
fn request_tab_reveal(
    tab_scroll: &ScrolledWindow,
    tab_strip: &GtkBox,
    reveal: &Rc<RefCell<Option<GtkBox>>>,
    pill: GtkBox,
) {
    *reveal.borrow_mut() = Some(pill);
    if !try_reveal_tab(tab_scroll, tab_strip, reveal) {
        tab_strip.queue_resize();
    }
}

fn wire_tab_reveal(
    tab_scroll: &ScrolledWindow,
    tab_strip: &GtkBox,
    reveal: &Rc<RefCell<Option<GtkBox>>>,
) {
    let scroll = tab_scroll.downgrade();
    let reveal_on_allocate = reveal.clone();
    tab_strip.connect_size_allocate(move |strip, _| {
        if let Some(scroll) = scroll.upgrade() {
            try_reveal_tab(&scroll, strip, &reveal_on_allocate);
        }
    });

    let scroll = tab_scroll.downgrade();
    let strip = tab_strip.downgrade();
    let reveal_on_change = reveal.clone();
    tab_scroll.hadjustment().connect_changed(move |_| {
        if let (Some(scroll), Some(strip)) = (scroll.upgrade(), strip.upgrade()) {
            try_reveal_tab(&scroll, &strip, &reveal_on_change);
        }
    });
}

/// Returns true once the pending pill (if any) is fully visible.
fn try_reveal_tab(
    tab_scroll: &ScrolledWindow,
    tab_strip: &GtkBox,
    reveal: &Rc<RefCell<Option<GtkBox>>>,
) -> bool {
    let Some(pill) = reveal.borrow().clone() else {
        return true;
    };
    if pill.parent().as_ref() != Some(tab_strip.upcast_ref::<gtk::Widget>()) {
        // The tab was closed before it could be revealed.
        reveal.borrow_mut().take();
        return true;
    }
    let allocation = pill.allocation();
    let Some((left, _)) = pill.translate_coordinates(tab_strip, 0, 0) else {
        return false;
    };
    if !pill.is_mapped() || allocation.width() <= 1 || left < 0 {
        return false;
    }
    let adjustment = tab_scroll.hadjustment();
    let page_size = adjustment.page_size();
    if page_size <= 0.0 {
        return false;
    }
    let width = allocation.width();
    let wanted = scroll_value_to_reveal(adjustment.value(), page_size, left, width);
    if (adjustment.value() - wanted).abs() > f64::EPSILON {
        adjustment.set_value(wanted);
    }
    let visible = pill_is_visible(adjustment.value(), page_size, left, width);
    if visible {
        reveal.borrow_mut().take();
    }
    visible
}

fn pill_is_visible(value: f64, page_size: f64, left: i32, width: i32) -> bool {
    let left = left as f64;
    let right = left + width as f64;
    left >= value - 0.5 && right <= value + page_size + 0.5
}

fn scroll_value_to_reveal(current: f64, page_size: f64, left: i32, width: i32) -> f64 {
    let left = left as f64;
    let right = left + width as f64;
    if left < current {
        left
    } else if right > current + page_size {
        right - page_size
    } else {
        current
    }
}

fn history_row(entry: &crate::history::HistoryEntry) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_widget_name(&entry.url);
    let label = Label::new(None);
    label.set_xalign(0.0);
    label.set_line_wrap(true);
    label.set_markup(&format!(
        "<b>{}</b>\n<small>{}</small>",
        glib::markup_escape_text(&entry.title),
        glib::markup_escape_text(&entry.url)
    ));
    label.set_margin_top(6);
    label.set_margin_bottom(6);
    label.set_margin_start(12);
    label.set_margin_end(12);
    row.add(&label);
    row
}

/// Shortens a tab title to at most `MAX` characters (not bytes: a 15-letter
/// Cyrillic title is 29 bytes and used to get an ellipsis) (X-19).
fn truncate(title: &str) -> String {
    const MAX: usize = 28;
    if title.chars().count() <= MAX {
        title.to_string()
    } else {
        let kept: String = title.chars().take(MAX - 1).collect();
        format!("{}…", kept.trim_end())
    }
}

pub fn build_chrome_layout(root: &gtk::Box) -> (TabChrome, Button) {
    let tab_bar = GtkBox::new(Orientation::Horizontal, 6);
    tab_bar.set_height_request(TAB_BAR_HEIGHT);
    tab_bar.style_context().add_class("chrome");
    tab_bar.style_context().add_class("tab-strip");

    let tab_scroll = ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    tab_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
    tab_scroll.set_shadow_type(gtk::ShadowType::None);
    tab_scroll.set_can_focus(false);

    let tab_strip = GtkBox::new(Orientation::Horizontal, 4);
    // Keyboard focus moving onto a clipped pill scrolls it into view (R-6).
    tab_strip.set_focus_hadjustment(&tab_scroll.hadjustment());
    tab_scroll.add(&tab_strip);
    tab_bar.pack_start(&tab_scroll, true, true, 0);

    let new_tab_btn = Button::new();
    let plus = Image::from_icon_name(Some("tab-new-symbolic"), gtk::IconSize::Button);
    new_tab_btn.set_image(Some(&plus));
    new_tab_btn.set_tooltip_text(Some("New Tab"));
    new_tab_btn.style_context().add_class("tab-new-btn");
    new_tab_btn.set_relief(gtk::ReliefStyle::None);
    new_tab_btn.set_focus_on_click(false);
    tab_bar.pack_start(&new_tab_btn, false, false, 0);

    root.pack_start(&tab_bar, false, false, 0);

    let sep1 = Separator::new(Orientation::Horizontal);
    sep1.style_context().add_class("chrome-separator");
    root.pack_start(&sep1, false, false, 0);

    let toolbar = GtkBox::new(Orientation::Horizontal, 6);
    toolbar.set_height_request(TOOLBAR_HEIGHT);
    toolbar.style_context().add_class("chrome");
    toolbar.style_context().add_class("toolbar");

    let home_btn = icon_button("go-home-symbolic", "Home");
    let back_btn = icon_button("go-previous-symbolic", "Back");
    let forward_btn = icon_button("go-next-symbolic", "Forward");
    let reload_btn = icon_button("view-refresh-symbolic", "Reload");

    toolbar.pack_start(&home_btn, false, false, 0);
    toolbar.pack_start(&back_btn, false, false, 0);
    toolbar.pack_start(&forward_btn, false, false, 0);
    toolbar.pack_start(&reload_btn, false, false, 0);

    let url_entry = gtk::Entry::new();
    url_entry.set_hexpand(true);
    url_entry.set_placeholder_text(Some("Search or type a URL…"));
    url_entry.style_context().add_class("url-bar");
    toolbar.pack_start(&url_entry, true, true, 0);

    root.pack_start(&toolbar, false, false, 0);

    let sep2 = Separator::new(Orientation::Horizontal);
    sep2.style_context().add_class("chrome-separator");
    root.pack_start(&sep2, false, false, 0);

    let find_bar = build_find_bar();
    root.pack_start(&find_bar.bar, false, false, 0);

    let stack = Stack::new();
    stack.style_context().add_class("void");
    stack.set_vexpand(true);
    root.pack_start(&stack, true, true, 0);

    let chrome = TabChrome {
        stack,
        tab_bar,
        tab_strip,
        tab_scroll,
        tab_separator: sep1,
        toolbar,
        toolbar_separator: sep2,
        url_entry,
        back_btn,
        forward_btn,
        reload_btn,
        home_btn,
        find_bar,
    };

    (chrome, new_tab_btn)
}

fn build_find_bar() -> FindBar {
    let bar = GtkBox::new(Orientation::Horizontal, 6);
    bar.style_context().add_class("chrome");
    bar.style_context().add_class("find-bar");

    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some("Find in page"));
    entry.set_width_chars(24);
    entry.style_context().add_class("find-entry");
    let status = Label::new(None);
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

fn icon_button(icon_name: &str, tooltip: &str) -> Button {
    let btn = Button::new();
    let img = Image::from_icon_name(Some(icon_name), gtk::IconSize::Button);
    btn.set_image(Some(&img));
    btn.set_tooltip_text(Some(tooltip));
    btn.style_context().add_class("ghost-btn");
    btn.set_relief(gtk::ReliefStyle::None);
    // Buttons take focus on click in GTK3, which left the :focus tint stuck on
    // Back/Forward/Reload after a mouse click (R-6). Keyboard users still
    // reach them with Tab.
    btn.set_focus_on_click(false);
    btn
}

#[cfg(test)]
mod tests {
    use super::{
        find_status_text, is_sized_popup, last_tab_action, next_tab_id, pill_is_visible,
        scroll_value_to_reveal, selection_after_close, shortcut_for, truncate, url_bar_sync_value,
        FindStatus, LastTab, Shortcut,
    };
    use super::{security_state, Security};
    use gdk::keys::constants as key;
    use gdk::ModifierType as M;

    #[test]
    fn tab_cycling_shortcuts_match_the_keyvals_gtk_reports() {
        assert_eq!(shortcut_for(&key::Tab, M::CONTROL_MASK), Some(Shortcut::NextTab));
        assert_eq!(
            shortcut_for(&key::Page_Down, M::CONTROL_MASK),
            Some(Shortcut::NextTab)
        );
        // GTK3 reports Shift+Tab as ISO_Left_Tab with Shift still in the state.
        assert_eq!(
            shortcut_for(&key::ISO_Left_Tab, M::CONTROL_MASK | M::SHIFT_MASK),
            Some(Shortcut::PreviousTab)
        );
        assert_eq!(
            shortcut_for(&key::Page_Up, M::CONTROL_MASK),
            Some(Shortcut::PreviousTab)
        );
        // Plain Tab must keep moving keyboard focus.
        assert_eq!(shortcut_for(&key::Tab, M::empty()), None);
        assert_eq!(shortcut_for(&key::ISO_Left_Tab, M::SHIFT_MASK), None);
    }

    #[test]
    fn shortcuts_ignore_lock_modifiers_and_letter_case() {
        // Num Lock (MOD2) and Caps Lock must not break shortcuts.
        let locks = M::MOD2_MASK | M::LOCK_MASK;
        assert_eq!(shortcut_for(&key::w, M::CONTROL_MASK | locks), Some(Shortcut::CloseTab));
        assert_eq!(shortcut_for(&key::T, M::CONTROL_MASK | locks), Some(Shortcut::NewTab));
        assert_eq!(shortcut_for(&key::F11, locks), Some(Shortcut::ToggleFullscreen));
        // Ctrl+Shift+T and plain letters are not ours.
        assert_eq!(shortcut_for(&key::t, M::CONTROL_MASK | M::SHIFT_MASK), None);
        assert_eq!(shortcut_for(&key::t, M::empty()), None);
    }

    #[test]
    fn number_shortcuts_select_tabs_by_position() {
        assert_eq!(shortcut_for(&key::_1, M::CONTROL_MASK), Some(Shortcut::SelectTab(0)));
        assert_eq!(shortcut_for(&key::_8, M::CONTROL_MASK), Some(Shortcut::SelectTab(7)));
        assert_eq!(shortcut_for(&key::_9, M::CONTROL_MASK), Some(Shortcut::LastTab));
        assert_eq!(shortcut_for(&key::Left, M::MOD1_MASK), Some(Shortcut::Back));
        assert_eq!(shortcut_for(&key::l, M::CONTROL_MASK), Some(Shortcut::FocusLocation));
    }

    #[test]
    fn only_window_open_with_a_size_gets_its_own_window() {
        let opener = Some((1200, 800));
        // window.open(url, name, "width=420,height=360")
        assert!(is_sized_popup((420, 360), opener, false));
        // window.open(url): WebKit reports the opener window's size.
        assert!(!is_sized_popup((1200, 800), opener, false));
        // target=_blank: WebCore's 100x100 minimum, but it is a link.
        assert!(!is_sized_popup((100, 100), opener, true));
        assert!(!is_sized_popup((0, 0), opener, false));
    }

    #[test]
    fn find_status_reports_match_counts_and_misses() {
        assert_eq!(find_status_text(FindStatus::Idle), "");
        assert_eq!(find_status_text(FindStatus::Found(1)), "1 match");
        assert_eq!(find_status_text(FindStatus::Found(3)), "3 matches");
        assert_eq!(find_status_text(FindStatus::Found(u32::MAX)), "1000+ matches");
        assert_eq!(find_status_text(FindStatus::NotFound), "No matches");
    }

    #[test]
    fn scroll_value_reveals_pills_outside_the_current_viewport() {
        assert_eq!(scroll_value_to_reveal(30.0, 100.0, 15, 40), 15.0);
        assert_eq!(scroll_value_to_reveal(30.0, 100.0, 120, 40), 60.0);
    }

    #[test]
    fn a_reveal_is_complete_only_when_the_whole_pill_is_inside_the_page() {
        assert!(pill_is_visible(0.0, 100.0, 10, 40));
        assert!(!pill_is_visible(0.0, 100.0, 90, 40));
        // Adjustment upper not yet grown: set_value was clamped short.
        let clamped_value = 60.0;
        assert!(!pill_is_visible(clamped_value, 100.0, 180, 40));
    }

    #[test]
    fn scroll_value_stays_put_when_the_pill_is_already_visible() {
        assert_eq!(scroll_value_to_reveal(20.0, 100.0, 40, 30), 20.0);
    }

    #[test]
    fn the_address_bar_marks_https_secure_and_remote_http_not_secure() {
        assert_eq!(security_state("https://example.com/", Some(false)), Security::Secure);
        assert_eq!(security_state("https://expired.example/", Some(true)), Security::Insecure);
        assert_eq!(security_state("https://example.com/", None), Security::None);
        assert_eq!(security_state("http://example.com/", None), Security::Insecure);
        assert_eq!(security_state("http://192.168.1.1/", None), Security::Insecure);
        assert_eq!(security_state("http://localhost:3000/", None), Security::None);
        assert_eq!(security_state("http://127.0.0.1:8011/", None), Security::None);
        assert_eq!(security_state("http://[::1]/", None), Security::None);
        assert_eq!(security_state("file:///etc/hostname", None), Security::None);
        assert_eq!(security_state("about:blank", None), Security::None);
    }

    #[test]
    fn tab_titles_are_truncated_by_characters_not_bytes() {
        // 15 characters, 29 bytes: short enough to show in full.
        assert_eq!(truncate("Новости Украины"), "Новости Украины");
        // 13 characters, 39 bytes.
        assert_eq!(truncate("维基百科，自由的百科全书"), "维基百科，自由的百科全书");
        let exact = "a".repeat(28);
        assert_eq!(truncate(&exact), exact);
        let long = "Википедия — свободная энциклопедия";
        let short = truncate(long);
        assert_eq!(short.chars().count(), 28);
        assert!(short.ends_with('…'));
        assert!(long.starts_with(short.trim_end_matches('…')));
    }

    #[test]
    fn closing_the_last_tab_replaces_it_with_home_unless_it_is_untouched() {
        assert_eq!(last_tab_action(1, true), LastTab::ReplaceWithHome);
        assert_eq!(last_tab_action(1, false), LastTab::KeepPristineHome);
        assert_eq!(last_tab_action(2, true), LastTab::NotLast);
        assert_eq!(last_tab_action(3, false), LastTab::NotLast);
    }

    #[test]
    fn closing_a_background_tab_keeps_the_selected_tab() {
        assert_eq!(selection_after_close(&[1, 3], Some(3), 2, 1), Some(3));
    }

    #[test]
    fn closing_the_selected_tab_selects_the_next_available_tab() {
        assert_eq!(selection_after_close(&[1, 3], Some(2), 2, 1), Some(3));
        assert_eq!(selection_after_close(&[1, 2], Some(3), 3, 2), Some(2));
    }

    #[test]
    fn uri_updates_preserve_text_while_the_url_bar_is_focused() {
        assert_eq!(url_bar_sync_value(true, "https://example.com"), None);
        assert_eq!(
            url_bar_sync_value(false, "https://example.com"),
            Some("https://example.com")
        );
    }

    #[test]
    fn keyboard_tab_switching_wraps_in_both_directions() {
        assert_eq!(next_tab_id(&[1, 2, 3], Some(2), false), Some(3));
        assert_eq!(next_tab_id(&[1, 2, 3], Some(3), false), Some(1));
        assert_eq!(next_tab_id(&[1, 2, 3], Some(1), true), Some(3));
        assert_eq!(next_tab_id(&[], None, false), None);
    }
}
