mod history;
mod home;
mod navigation;
mod profile;
mod tabs;
mod theme;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Orientation};
use history::HistoryStore;
use profile::{app_data_dir, persistent_web_context};
use std::cell::RefCell;
use std::rc::Rc;
use tabs::{build_chrome_layout, TabManager};

pub const START_PAGE: &str = "https://duckduckgo.com/";
const APP_ID: &str = "com.nodaysidle.Browser";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| theme::install());
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let data_dir = app_data_dir();
    let web_context = persistent_web_context(&data_dir);
    let history = Rc::new(RefCell::new(HistoryStore::load(data_dir.join("history.json"))));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("nodaysidle")
        .default_width(1_200)
        .default_height(800)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 0);
    window.add(&root);

    let (chrome, new_tab_btn) = build_chrome_layout(&root);
    let tab_manager = TabManager::new(chrome, web_context, history.clone());
    tab_manager.wire_toolbar(&new_tab_btn);
    tab_manager.wire_history(history);

    tab_manager.open_initial_home();
    window.show_all();
}
