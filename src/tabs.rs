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
}

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
}

impl TabManager {
    pub fn new(
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
        };
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
    }

    pub fn wire_keyboard(&self, window: &gtk::ApplicationWindow) {
        let group = gtk::AccelGroup::new();
        window.add_accel_group(&group);

        let mgr = self.inner.clone();
        group.connect_accel_group(
            *gdk::keys::constants::t,
            gdk::ModifierType::CONTROL_MASK,
            gtk::AccelFlags::VISIBLE,
            move |_, _, _, _| {
                TabManager::open_tab(&mgr, TabOpen::Home, true);
                true
            },
        );

        let mgr = self.inner.clone();
        group.connect_accel_group(
            *gdk::keys::constants::w,
            gdk::ModifierType::CONTROL_MASK,
            gtk::AccelFlags::VISIBLE,
            move |_, _, _, _| {
                let selected = { mgr.borrow().selected };
                if let Some(tab_id) = selected {
                    TabManager::close_tab(&mgr, tab_id);
                }
                true
            },
        );

        let url_entry = self.inner.borrow().url_entry.clone();
        group.connect_accel_group(
            *gdk::keys::constants::l,
            gdk::ModifierType::CONTROL_MASK,
            gtk::AccelFlags::VISIBLE,
            move |_, _, _, _| {
                url_entry.grab_focus();
                url_entry.select_region(0, -1);
                true
            },
        );

        let mgr = self.inner.clone();
        group.connect_accel_group(
            *gdk::keys::constants::Tab,
            gdk::ModifierType::CONTROL_MASK,
            gtk::AccelFlags::VISIBLE,
            move |_, _, _, _| {
                cycle_selected_tab(&mgr, false);
                true
            },
        );

        let mgr = self.inner.clone();
        group.connect_accel_group(
            *gdk::keys::constants::Tab,
            gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
            gtk::AccelFlags::VISIBLE,
            move |_, _, _, _| {
                cycle_selected_tab(&mgr, true);
                true
            },
        );

        let mgr = self.inner.clone();
        let window_for_find = window.clone();
        group.connect_accel_group(
            *gdk::keys::constants::f,
            gdk::ModifierType::CONTROL_MASK,
            gtk::AccelFlags::VISIBLE,
            move |_, _, _, _| {
                show_find_dialog(&mgr, &window_for_find);
                true
            },
        );

        let mgr = self.inner.clone();
        let window_for_fullscreen = window.clone();
        group.connect_accel_group(
            *gdk::keys::constants::F11,
            gdk::ModifierType::empty(),
            gtk::AccelFlags::VISIBLE,
            move |_, _, _, _| {
                let is_fullscreen = window_for_fullscreen
                    .window()
                    .map(|window| window.state().contains(gdk::WindowState::FULLSCREEN))
                    .unwrap_or(false);
                if is_fullscreen {
                    window_for_fullscreen.unfullscreen();
                    set_browser_chrome_visible(&mgr, true);
                } else {
                    set_browser_chrome_visible(&mgr, false);
                    window_for_fullscreen.fullscreen();
                }
                true
            },
        );
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
        close_btn.set_can_focus(false);

        let title_hit = EventBox::new();
        title_hit.add(&title_label);
        title_hit.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
        title_hit.set_can_focus(true);
        title_hit.set_focus_on_click(true);
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
        TabManager::create_webview(mgr, tab_id, None)
    }

    fn create_webview(
        mgr: &Rc<RefCell<TabManagerInner>>,
        tab_id: u32,
        related_view: Option<&WebView>,
    ) -> Option<WebView> {
        let (web_context, history, page_stack) = {
            let inner = mgr.borrow();
            let tab = inner.tabs.iter().find(|tab| tab.id == tab_id)?;
            if let Some(view) = &tab.webview {
                return Some(view.clone());
            }
            (
                inner.web_context.clone(),
                inner.history.clone(),
                tab.page_stack.clone(),
            )
        };

        let webview = related_view
            .map(WebView::with_related_view)
            .unwrap_or_else(|| WebView::with_context(&web_context));
        if let Some(settings) = WebViewExt::settings(&webview) {
            settings.set_enable_javascript(true);
            settings.set_enable_html5_database(true);
            settings.set_enable_html5_local_storage(true);
        }
        crate::permissions::wire(&webview);

        let mgr_load = mgr.clone();

        webview.connect_load_changed(clone!(@strong mgr_load, @strong history => move |view, ev| {
            if ev != LoadEvent::Finished {
                return;
            }
            let uri = view.uri().unwrap_or_default();
            if uri.is_empty() || uri == "about:blank" {
                return;
            }
            let title = title_for_page(view.title().as_deref(), &uri);
            history.borrow_mut().record(uri.to_string(), title.clone());
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

        let (tab_bar, tab_separator, toolbar, toolbar_separator) = {
            let inner = mgr.borrow();
            (
                inner.tab_bar.clone(),
                inner.tab_separator.clone(),
                inner.toolbar.clone(),
                inner.toolbar_separator.clone(),
            )
        };
        let tab_bar_on_enter = tab_bar.clone();
        let tab_separator_on_enter = tab_separator.clone();
        let toolbar_on_enter = toolbar.clone();
        let toolbar_separator_on_enter = toolbar_separator.clone();
        webview.connect_enter_fullscreen(move |view| {
            let Some(window) = view
                .toplevel()
                .and_then(|widget| widget.downcast::<gtk::Window>().ok())
            else {
                return false;
            };
            tab_bar_on_enter.hide();
            tab_separator_on_enter.hide();
            toolbar_on_enter.hide();
            toolbar_separator_on_enter.hide();
            window.fullscreen();
            true
        });

        webview.connect_leave_fullscreen(move |view| {
            let Some(window) = view
                .toplevel()
                .and_then(|widget| widget.downcast::<gtk::Window>().ok())
            else {
                return false;
            };
            window.unfullscreen();
            tab_bar.show();
            tab_separator.show();
            toolbar.show();
            toolbar_separator.show();
            true
        });

        let mgr_create = mgr.clone();
        webview.connect_create(move |parent, _action| {
            TabManager::open_related_tab(&mgr_create, parent).map(|view| view.upcast())
        });

        let mgr_close = mgr.clone();
        webview.connect_close(move |_| {
            TabManager::close_tab(&mgr_close, tab_id);
        });

        webview.connect_ready_to_show(|view| view.show());

        {
            let mut inner = mgr.borrow_mut();
            let tab = inner.tabs.iter_mut().find(|tab| tab.id == tab_id)?;
            tab.webview = Some(webview.clone());
        }
        page_stack.add_named(&webview, "web");
        webview.show_all();
        page_stack.set_visible_child_name("web");
        Some(webview)
    }

    fn open_related_tab(
        mgr: &Rc<RefCell<TabManagerInner>>,
        related_view: &WebView,
    ) -> Option<WebView> {
        let tab_id = TabManager::open_tab(mgr, TabOpen::Home, true);
        let webview = TabManager::create_webview(mgr, tab_id, Some(related_view))?;
        TabManager::select_tab_id(mgr, tab_id);
        Some(webview)
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
        let (stack, url_entry, tab_scroll, tab_strip, sync) = {
            let mut inner = mgr.borrow_mut();
            inner.selected = Some(id);
            let show_close = inner.tabs.len() > 1;
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
                tab.close_btn.set_visible(show_close);
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
                sync,
            )
        };

        stack.set_visible_child_name(&id.to_string());
        if let Some((pill, page_stack, webview, home_search, title_label)) = sync {
            schedule_scroll_tab_into_view(tab_scroll, tab_strip, pill);
            let on_home = page_stack.visible_child_name().as_deref() == Some("home")
                || webview.is_none();
            if on_home {
                page_stack.set_visible_child_name("home");
                url_entry.set_text("");
                title_label.set_text("New Tab");
                home_search.grab_focus();
            } else if let Some(view) = webview {
                page_stack.set_visible_child_name("web");
                let uri = view.uri().unwrap_or_default();
                url_entry.set_text(&uri);
                view.grab_focus();
            }
        }
        mgr.borrow().refresh_nav_buttons();
    }

    fn close_tab(mgr: &Rc<RefCell<TabManagerInner>>, id: u32) {
        let (removed_selected, select_after) = {
            let mut inner = mgr.borrow_mut();
            if inner.tabs.len() <= 1 {
                return;
            }
            let Some(idx) = inner.tabs.iter().position(|t| t.id == id) else {
                return;
            };
            let tab = inner.tabs.remove(idx);
            inner.stack.remove(&tab.page_stack);
            inner.tab_strip.remove(&tab.pill);
            let was_selected = inner.selected == Some(id);
            let remaining_ids = inner.tabs.iter().map(|tab| tab.id).collect::<Vec<_>>();
            let selected_after = selection_after_close(&remaining_ids, inner.selected, id, idx);
            let select_after = if was_selected {
                inner.selected = None;
                selected_after
            } else {
                None
            };
            (was_selected, select_after)
        };

        if let Some(next_id) = select_after {
            TabManager::select_tab_id(mgr, next_id);
        } else if !removed_selected {
            // Refresh close buttons visibility
            let mgr2 = mgr.clone();
            glib::idle_add_local(move || {
                let selected = { mgr2.borrow().selected };
                if let Some(id) = selected {
                    TabManager::select_tab_id(&mgr2, id);
                }
                glib::ControlFlow::Break
            });
        }
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

fn url_bar_sync_value(is_focused: bool, uri: &str) -> Option<&str> {
    (!is_focused).then_some(uri)
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

fn show_find_dialog(mgr: &Rc<RefCell<TabManagerInner>>, window: &gtk::ApplicationWindow) {
    let view = { mgr.borrow().selected_webview() };
    let Some(controller) = view.and_then(|view| view.find_controller()) else {
        return;
    };

    let dialog = gtk::Dialog::with_buttons(
        Some("Find in page"),
        Some(window.upcast_ref::<gtk::Window>()),
        gtk::DialogFlags::DESTROY_WITH_PARENT,
        &[],
    );
    dialog.set_modal(false);

    let content = dialog.content_area();
    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some("Find text"));
    entry.set_margin_top(12);
    entry.set_margin_start(12);
    entry.set_margin_end(12);
    content.pack_start(&entry, false, false, 0);

    let buttons = GtkBox::new(Orientation::Horizontal, 6);
    buttons.set_halign(gtk::Align::End);
    buttons.set_margin_top(8);
    buttons.set_margin_bottom(12);
    buttons.set_margin_start(12);
    buttons.set_margin_end(12);
    let previous = Button::with_label("Previous");
    let next = Button::with_label("Next");
    let close = Button::with_label("Close");
    buttons.pack_start(&previous, false, false, 0);
    buttons.pack_start(&next, false, false, 0);
    buttons.pack_start(&close, false, false, 0);
    content.pack_start(&buttons, false, false, 0);

    let find_on_change = controller.clone();
    entry.connect_changed(move |entry| {
        let text = entry.text();
        if text.is_empty() {
            find_on_change.search_finish();
        } else {
            find_on_change.search(
                text.as_str(),
                (FindOptions::CASE_INSENSITIVE | FindOptions::WRAP_AROUND).bits(),
                1_000,
            );
        }
    });

    let find_on_activate = controller.clone();
    entry.connect_activate(move |_| find_on_activate.search_next());

    let find_previous = controller.clone();
    previous.connect_clicked(move |_| find_previous.search_previous());

    let find_next = controller.clone();
    next.connect_clicked(move |_| find_next.search_next());

    let dialog_weak = dialog.downgrade();
    close.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    });

    let finish_on_destroy = controller.clone();
    dialog.connect_destroy(move |_| finish_on_destroy.search_finish());

    let dialog_weak = dialog.downgrade();
    let finish_on_escape = controller;
    entry.connect_key_press_event(move |_, event| {
        if event.keyval() == gdk::keys::constants::Escape {
            finish_on_escape.search_finish();
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.close();
            }
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });

    dialog.show_all();
    entry.grab_focus();
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

fn schedule_scroll_tab_into_view(tab_scroll: ScrolledWindow, tab_strip: GtkBox, pill: GtkBox) {
    glib::idle_add_local(move || {
        if let Some((left, _)) = pill.translate_coordinates(&tab_strip, 0, 0) {
            let width = pill.allocated_width();
            if width > 0 {
                let adjustment = tab_scroll.hadjustment();
                let value = scroll_value_to_reveal(
                    adjustment.value(),
                    adjustment.page_size(),
                    left,
                    width,
                );
                adjustment.set_value(value);
            }
        }
        glib::ControlFlow::Break
    });
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

fn truncate(title: &str) -> String {
    const MAX: usize = 28;
    if title.len() <= MAX {
        title.to_string()
    } else {
        format!("{}…", title.chars().take(MAX - 1).collect::<String>())
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
    tab_scroll.add(&tab_strip);
    tab_bar.pack_start(&tab_scroll, true, true, 0);

    let new_tab_btn = Button::new();
    let plus = Image::from_icon_name(Some("tab-new-symbolic"), gtk::IconSize::Button);
    new_tab_btn.set_image(Some(&plus));
    new_tab_btn.set_tooltip_text(Some("New Tab"));
    new_tab_btn.style_context().add_class("tab-new-btn");
    new_tab_btn.set_relief(gtk::ReliefStyle::None);
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
    };

    (chrome, new_tab_btn)
}

fn icon_button(icon_name: &str, tooltip: &str) -> Button {
    let btn = Button::new();
    let img = Image::from_icon_name(Some(icon_name), gtk::IconSize::Button);
    btn.set_image(Some(&img));
    btn.set_tooltip_text(Some(tooltip));
    btn.style_context().add_class("ghost-btn");
    btn.set_relief(gtk::ReliefStyle::None);
    btn
}

#[cfg(test)]
mod tests {
    use super::{
        next_tab_id, scroll_value_to_reveal, selection_after_close, url_bar_sync_value,
    };

    #[test]
    fn scroll_value_reveals_pills_outside_the_current_viewport() {
        assert_eq!(scroll_value_to_reveal(30.0, 100.0, 15, 40), 15.0);
        assert_eq!(scroll_value_to_reveal(30.0, 100.0, 120, 40), 60.0);
    }

    #[test]
    fn scroll_value_stays_put_when_the_pill_is_already_visible() {
        assert_eq!(scroll_value_to_reveal(20.0, 100.0, 40, 30), 20.0);
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
