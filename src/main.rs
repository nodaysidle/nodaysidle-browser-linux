mod history;
mod home;
mod icon;
mod downloads;
mod error_page;
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
use std::path::Path;
use std::rc::Rc;
use tabs::{build_chrome_layout, TabManager};

const APP_ID: &str = "com.nodaysidle.Browser";

fn main() -> glib::ExitCode {
    // GTK 3 uses the program name as the Wayland app_id and as X11's WM_CLASS
    // instance, and the program class as the WM_CLASS class. Set both to the
    // application ID so the window, com.nodaysidle.Browser.desktop and its
    // StartupWMClass all match however the binary is invoked (X-26). GDK must
    // be initialized before the class can be set, hence the startup handler.
    glib::set_prgname(Some(APP_ID));
    glib::set_application_name("nodaysidle");
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    let tab_manager = Rc::new(RefCell::new(None::<TabManager>));
    app.connect_startup(|_| {
        gdk::set_program_class(APP_ID);
        theme::install();
        icon::install_default_icon();
    });
    let manager_for_shutdown = tab_manager.clone();
    app.connect_shutdown(move |_| {
        let manager = { manager_for_shutdown.borrow().clone() };
        if let Some(manager) = manager {
            manager.flush_history();
        }
    });
    let manager_for_activate = tab_manager.clone();
    app.connect_activate(move |app| {
        // A second launch activates the primary instance: bring its window
        // to the front instead of doing nothing visible (R-4).
        get_or_build_ui(app, &manager_for_activate).present();
    });
    app.connect_open(move |app, files, _hint| {
        let uris = files
            .iter()
            .filter_map(|file| external_uri_to_open(file.uri().as_str()))
            .collect::<Vec<_>>();
        let manager = get_or_build_ui(app, &tab_manager);
        for uri in uris {
            manager.open_external_uri(&uri);
        }
        manager.present();
    });
    let args = command_line_args(std::env::args().collect(), |path| path.exists());
    app.run_with_args(&args)
}

/// GApplication turns every non-URI argument into a file relative to the
/// working directory, so `nodaysidle-browser wikipedia.org` used to open
/// file:///<cwd>/wikipedia.org (R-10). Resolve arguments here instead: an
/// existing file opens as a file, and anything else goes through the same
/// resolution as the address bar.
fn command_line_args(args: Vec<String>, exists: impl Fn(&Path) -> bool) -> Vec<String> {
    let cwd = std::env::current_dir().ok();
    let mut options_done = false;
    args.into_iter()
        .enumerate()
        .map(|(index, arg)| {
            if index == 0 {
                return arg;
            }
            if !options_done && arg.starts_with('-') {
                options_done = arg == "--";
                return arg;
            }
            command_line_target(&arg, cwd.as_deref(), &exists).unwrap_or(arg)
        })
        .collect()
}

fn command_line_target(
    arg: &str,
    cwd: Option<&Path>,
    exists: &impl Fn(&Path) -> bool,
) -> Option<String> {
    let path = Path::new(arg);
    let absolute = match cwd {
        Some(cwd) if path.is_relative() => cwd.join(path),
        _ => path.to_path_buf(),
    };
    let looks_like_path = arg.starts_with("./") || arg.starts_with("../");
    if exists(&absolute) || (looks_like_path && cwd.is_some()) {
        let absolute = absolute.canonicalize().unwrap_or(absolute);
        return url::Url::from_file_path(&absolute).ok().map(String::from);
    }
    navigation::resolve(arg, navigation::SearchEngine::default())
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

    window.style_context().add_class("browser-window");
    let root = GtkBox::new(Orientation::Vertical, 0);
    window.add(&root);

    let (chrome, new_tab_btn) = build_chrome_layout(&root);
    let tab_manager = TabManager::new(&window, chrome, web_context, history.clone());
    tab_manager.wire_toolbar(&new_tab_btn);
    tab_manager.wire_keyboard(&window);
    tab_manager.wire_app_menu();
    tab_manager.wire_history(history);

    tab_manager.open_initial_home();
    window.show_all();
    tab_manager
}

fn external_uri_to_open(uri: &str) -> Option<String> {
    let parsed = url::Url::parse(uri).ok()?;
    match parsed.scheme() {
        "http" | "https" | "file" | "about" => Some(parsed.into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::external_uri_to_open;
    use super::get_or_insert_manager;
    use super::{command_line_args, command_line_target};
    use std::cell::{Cell, RefCell};
    use std::path::Path;

    fn target(arg: &str, existing: &[&str]) -> Option<String> {
        command_line_target(arg, Some(Path::new("/work")), &|path: &Path| {
            existing.iter().any(|existing| path == Path::new(existing))
        })
    }

    #[test]
    fn bare_host_arguments_resolve_like_the_address_bar() {
        assert_eq!(target("wikipedia.org", &[]), Some("https://wikipedia.org".into()));
        assert_eq!(target("localhost:3000", &[]), Some("http://localhost:3000".into()));
        assert_eq!(
            target("https://example.com/a", &[]),
            Some("https://example.com/a".into())
        );
    }

    #[test]
    fn existing_files_and_explicit_paths_open_as_files() {
        assert_eq!(
            target("wikipedia.org", &["/work/wikipedia.org"]),
            Some("file:///work/wikipedia.org".into())
        );
        assert_eq!(target("./missing.html", &[]), Some("file:///work/missing.html".into()));
        assert_eq!(target("/tmp/page.html", &[]), Some("file:///tmp/page.html".into()));
    }

    #[test]
    fn options_and_program_name_are_left_alone() {
        let args = ["prog", "--gapplication-service", "wikipedia.org", "--", "-dash.org"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            command_line_args(args, |_| false),
            ["prog", "--gapplication-service", "https://wikipedia.org", "--", "https://-dash.org"]
        );
    }

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
    fn external_about_blank_is_accepted() {
        assert_eq!(external_uri_to_open("about:blank"), Some("about:blank".to_string()));
    }

    #[test]
    fn external_uri_open_rejects_unsupported_schemes_and_invalid_input() {
        assert_eq!(external_uri_to_open("javascript:alert(1)"), None);
        assert_eq!(external_uri_to_open("data:text/html,hello"), None);
        assert_eq!(external_uri_to_open("not a URI"), None);
    }
}
