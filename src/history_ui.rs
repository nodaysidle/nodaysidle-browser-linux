//! History popover construction and its dynamic narrow-window sizing (A8).
//!
//! The popover is rebuilt every time it opens so it always shows current
//! entries, and its requested size is bounded by the browser window so it
//! stays usable in a narrow Hyprland tile.

use crate::history::HistoryStore;
use crate::tabs::TabManager;
use glib::clone;
use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Button, Image, Label, ListBox, ListBoxRow, Orientation, Popover, ScrolledWindow,
    Separator,
};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn wire_history(manager: &TabManager, history: Rc<RefCell<HistoryStore>>) {
    let history_btn = Button::new();
    let img = Image::from_icon_name(Some("document-open-recent-symbolic"), gtk::IconSize::Button);
    history_btn.set_image(Some(&img));
    history_btn.set_tooltip_text(Some("History"));
    history_btn.style_context().add_class("ghost-btn");
    history_btn.set_relief(gtk::ReliefStyle::None);
    history_btn.set_focus_on_click(false);

    let url_entry = manager.inner.borrow().url_entry.clone();
    if let Some(toolbar) = url_entry.parent().and_then(|p| p.downcast::<GtkBox>().ok()) {
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
    scroll.set_min_content_height(200);
    scroll.set_max_content_height(320);
    let list = ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    scroll.add(&list);
    box_.pack_start(&scroll, true, true, 0);
    popover.add(&box_);

    let mgr = manager.clone_handle();
    history_btn.connect_clicked(
        clone!(@weak popover, @weak list, @weak scroll, @strong history, @strong mgr => move |_| {
            let window = { mgr.inner.borrow().window.clone() };
            // Bound the popover to the window so it stays usable in a narrow
            // Hyprland tile instead of overflowing it (A8).
            let width = (window.allocated_width() - 32).clamp(240, 420);
            let height = (window.allocated_height() - 96).clamp(200, 320);
            scroll.set_min_content_width(width);
            scroll.set_min_content_height(height);
            scroll.set_max_content_height(height);
            while let Some(row) = list.row_at_index(0) {
                list.remove(&row);
            }
            let entries = { history.borrow().entries().to_vec() };
            if entries.is_empty() {
                list.add(&history_empty_row());
            } else {
                for entry in &entries {
                    list.add(&history_row(entry));
                }
            }
            popover.show_all();
            popover.popup();
        }),
    );

    list.connect_row_activated(clone!(@weak popover, @strong mgr => move |_, row| {
        let url = row.widget_name().to_string();
        if url.is_empty() {
            return;
        }
        popover.popdown();
        mgr.navigate_from_bar_public(&url);
    }));
}

pub(crate) fn history_row(entry: &crate::history::HistoryEntry) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_widget_name(&entry.url);
    let label = Label::new(None);
    label.set_xalign(0.0);
    label.set_line_wrap(true);
    label.set_markup(&history_row_markup(entry));
    label.set_margin_top(6);
    label.set_margin_bottom(6);
    label.set_margin_start(12);
    label.set_margin_end(12);
    row.add(&label);
    row
}

/// A non-selectable, centered placeholder shown when history is empty.
pub(crate) fn history_empty_row() -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_selectable(false);
    row.set_activatable(false);
    let empty = Label::new(Some("No browsing history yet"));
    empty.set_halign(gtk::Align::Center);
    empty.set_valign(gtk::Align::Center);
    empty.set_vexpand(true);
    empty.set_margin_top(24);
    empty.set_margin_bottom(24);
    empty.set_margin_start(12);
    empty.set_margin_end(12);
    row.add(&empty);
    row
}

pub(crate) fn history_row_markup(entry: &crate::history::HistoryEntry) -> String {
    format!(
        "<b>{}</b>\n<small>{}</small>",
        glib::markup_escape_text(&entry.title),
        glib::markup_escape_text(&crate::navigation::format_url_for_display(&entry.url))
    )
}
