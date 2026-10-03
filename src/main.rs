mod history;
mod home;
mod downloads;
mod navigation;
mod permissions;
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
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    let tab_manager = Rc::new(RefCell::new(None::<TabManager>));
    app.connect_startup(|_| theme::install());
    let manager_for_activate = tab_manager.clone();
    app.connect_activate(move |app| {
        get_or_build_ui(app, &manager_for_activate);
    });
    app.connect_open(move |app, files, _hint| {
        let uris = files
            .iter()
            .filter_map(|file| external_uri_to_open(file.uri().as_str()))
            .collect::<Vec<_>>();
        let manager = get_or_build_ui(app, &tab_manager);
        if uris.is_empty() {
            return;
        }
        for uri in uris {
            manager.open_external_uri(&uri);
        }
    });
    app.run()
}

fn get_or_build_ui(
    app: &Application,
    manager_slot: &Rc<RefCell<Option<TabManager>>>,
) -> TabManager {
    get_or_insert_manager(manager_slot, || build_ui(app))
}

fn get_or_insert_manager<T: Clone>(slot: &RefCell<Option<T>>, build: impl FnOnce() -> T) -> T {
    let existing = { slot.borrow().clone() };
    if let Some(value) = existing {
        return value;
    }
    let value = build();
    *slot.borrow_mut() = Some(value.clone());
    value
}

fn build_ui(app: &Application) -> TabManager {
    let data_dir = app_data_dir();
    let web_context = persistent_web_context(&data_dir);
    downloads::wire(&web_context);
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
    tab_manager.wire_keyboard(&window);
    tab_manager.wire_history(history);

    tab_manager.open_initial_home();
    window.show_all();
    tab_manager
}

fn external_uri_to_open(uri: &str) -> Option<String> {
    let parsed = url::Url::parse(uri).ok()?;
    match parsed.scheme() {
        "http" | "https" | "file" => Some(parsed.into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::external_uri_to_open;
    use super::get_or_insert_manager;
    use std::cell::{Cell, RefCell};

    #[test]
    fn repeated_application_activation_reuses_the_existing_manager() {
        let slot = RefCell::new(None);
        let builds = Cell::new(0);
        let first = get_or_insert_manager(&slot, || {
            builds.set(builds.get() + 1);
            7
        });
        let second = get_or_insert_manager(&slot, || {
            builds.set(builds.get() + 1);
            9
        });

        assert_eq!(first, 7);
        assert_eq!(second, 7);
        assert_eq!(builds.get(), 1);
    }

    #[test]
    fn external_http_https_and_html_file_uris_are_accepted() {
        assert_eq!(
            external_uri_to_open("https://example.com/path"),
            Some("https://example.com/path".to_string())
        );
        assert_eq!(
            external_uri_to_open("http://localhost:3000/"),
            Some("http://localhost:3000/".to_string())
        );
        assert_eq!(
            external_uri_to_open("file:///tmp/page.html"),
            Some("file:///tmp/page.html".to_string())
        );
    }

    #[test]
    fn external_uri_open_rejects_unsupported_schemes_and_invalid_input() {
        assert_eq!(external_uri_to_open("javascript:alert(1)"), None);
        assert_eq!(external_uri_to_open("data:text/html,hello"), None);
        assert_eq!(external_uri_to_open("not a URI"), None);
    }
}
