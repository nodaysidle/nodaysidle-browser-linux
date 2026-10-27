mod assistant;
mod downloads;
mod error_page;
mod find;
mod history;
mod history_ui;
mod home;
mod icon;
mod navigation;
mod permissions;
mod popup;
mod privacy;
mod profile;
mod shortcuts;
mod tabs;
mod theme;
mod url_display;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Orientation};
use history::HistoryStore;
use profile::{app_data_dir, persistent_web_context};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use tabs::{build_chrome_layout, TabManager};

const APP_ID: &str = "com.nodaysidle.Browser";

/// Profile state shared by every browser window in one process: a single
/// WebKit `WebContext` (so cookies and site storage are common to all windows)
/// and one `HistoryStore` (so history is written once). Built on the first
/// window and reused by every window opened afterwards.
struct SharedProfile {
    web_context: webkit2gtk::WebContext,
    history: Rc<RefCell<HistoryStore>>,
}

/// `None` until the first window initialises the profile.
type SharedProfileSlot = Rc<RefCell<Option<Rc<SharedProfile>>>>;
/// Every open main browser window. Closed windows are pruned on destroy.
type WindowList = Rc<RefCell<Vec<TabManager>>>;

fn main() -> glib::ExitCode {
    #[cfg(target_os = "linux")]
    {
        // Disable DMA-BUF renderer by default to prevent Mesa/GBM teardown segfault on Intel/Wayland
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    // GTK 3 uses the program name as the Wayland app_id and as X11's WM_CLASS
    // instance, and the program class as the WM_CLASS class. Set both to the
    // application ID so the window, com.nodaysidle.Browser.desktop and its
    // StartupWMClass all match however the binary is invoked (X-26). GDK must
    // be initialized before the class can be set, hence the startup handler.
    glib::set_prgname(Some(APP_ID));
    glib::set_application_name("nodaysidle");
    // HANDLES_COMMAND_LINE (rather than HANDLES_OPEN) so `--new-window` and the
    // target arguments reach the primary instance verbatim when a second
    // launch is forwarded to it; that is how `--new-window` and Ctrl+N share
    // one WebContext and one HistoryStore.
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    let shared: SharedProfileSlot = Rc::new(RefCell::new(None));
    let windows: WindowList = Rc::new(RefCell::new(Vec::new()));

    app.connect_startup(|_| {
        gdk::set_program_class(APP_ID);
        theme::install();
        icon::install_default_icon();
    });

    let shared_for_shutdown = shared.clone();
    app.connect_shutdown(move |_| {
        flush_shared_history(&shared_for_shutdown);
        profile::remove_session_only_dir();
    });
    for &sig in &[1, 2, 15] {
        let shared_for_sig = shared.clone();
        let app_for_sig = app.clone();
        glib::unix_signal_add_local(sig, move || {
            flush_shared_history(&shared_for_sig);
            app_for_sig.quit();
            glib::ControlFlow::Break
        });
    }

    // Activation (e.g. clicking the dock icon) ensures a window exists and
    // raises the most recent one.
    let windows_for_activate = windows.clone();
    let shared_for_activate = shared.clone();
    app.connect_activate(move |app| {
        let existing = { windows_for_activate.borrow().last().cloned() };
        match existing {
            Some(manager) => manager.present(),
            None => {
                let _ = build_window(app, &windows_for_activate, &shared_for_activate);
            }
        }
    });

    let windows_for_cmd = windows.clone();
    let shared_for_cmd = shared.clone();
    app.connect_command_line(move |app, command_line| {
        let args = command_line
            .arguments()
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let cwd = std::env::current_dir().ok();
        let options = command_line_options(&args, cwd.as_deref(), &|path: &Path| path.exists());
        open_command_line(app, &windows_for_cmd, &shared_for_cmd, &options);
        0
    });

    let args = std::env::args().collect::<Vec<String>>();
    app.run_with_args(&args)
}

fn flush_shared_history(shared: &SharedProfileSlot) {
    let history = {
        shared
            .borrow()
            .as_ref()
            .map(|profile| profile.history.clone())
    };
    if let Some(history) = history {
        history.borrow_mut().flush();
    }
}

/// Flags and targets parsed from a GApplication command line. `--new-window`
/// is handled here (not as a GApplication option) so it survives the handoff
/// to the primary instance when a second launch forwards its command line.
#[derive(Debug, PartialEq, Eq)]
struct CommandLineOptions {
    new_window: bool,
    targets: Vec<String>,
}

fn command_line_options(
    args: &[String],
    cwd: Option<&Path>,
    exists: &impl Fn(&Path) -> bool,
) -> CommandLineOptions {
    let mut options = CommandLineOptions {
        new_window: false,
        targets: Vec::new(),
    };
    // `args[0]` is the program name; `--` is already consumed by GApplication.
    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--new-window" | "-n" => options.new_window = true,
            // Unknown options are ignored rather than treated as destinations.
            _ if arg.starts_with('-') => {}
            _ => {
                if let Some(uri) = command_line_target(arg, cwd, exists) {
                    options.targets.push(uri);
                }
            }
        }
    }
    options
}

fn open_command_line(
    app: &Application,
    windows: &WindowList,
    shared: &SharedProfileSlot,
    options: &CommandLineOptions,
) {
    // `--new-window` always opens a fresh window; otherwise the most recent
    // window is reused (or created when none exists). The borrow that reads the
    // list ends before `build_window` mutates it.
    let existing = windows.borrow().last().cloned();
    let manager = if options.new_window {
        build_window(app, windows, shared)
    } else {
        match existing {
            Some(manager) => Some(manager),
            None => build_window(app, windows, shared),
        }
    };
    let Some(manager) = manager else {
        return;
    };
    for uri in &options.targets {
        manager.open_external_uri(uri);
    }
    manager.present();
}

/// Whether closing a window must ask about cancelling downloads. Only the last
/// main browser window asks; closing an extra window keeps them running.
fn should_confirm_quit(is_last_window: bool, has_active_downloads: bool) -> bool {
    is_last_window && has_active_downloads
}

/// Initialises the shared profile on first use and builds a new browser window
/// on top of it.
fn build_window(
    app: &Application,
    windows: &WindowList,
    shared: &SharedProfileSlot,
) -> Option<TabManager> {
    let profile = ensure_profile(app, shared)?;

    let window = ApplicationWindow::builder()
        .application(app)
        .title("nodaysidle")
        .default_width(1_200)
        .default_height(800)
        .build();

    // Downloads are shared across windows through the WebContext, so only the
    // last main window needs to confirm cancellation (V-2).
    let windows_for_delete = windows.clone();
    window.connect_delete_event(move |window, _| {
        let is_last_window = windows_for_delete.borrow().len() <= 1;
        if should_confirm_quit(is_last_window, downloads::has_active_downloads()) {
            if downloads::confirm_quit(window.upcast_ref()) {
                downloads::cancel_all_active();
                glib::Propagation::Proceed
            } else {
                glib::Propagation::Stop
            }
        } else {
            glib::Propagation::Proceed
        }
    });

    window.style_context().add_class("browser-window");
    let frame = GtkBox::new(Orientation::Vertical, 0);
    frame.style_context().add_class("browser-frame");
    theme::apply_browser_frame_insets(&frame, "nodaysidle-browser-frame");
    theme::wire_window_corner_insets(window.upcast_ref(), &frame);
    window.add(&frame);

    let (chrome, new_tab_btn) = build_chrome_layout(&frame);
    let tab_manager = TabManager::new(
        &window,
        chrome,
        profile.web_context.clone(),
        profile.history.clone(),
    );
    tab_manager.wire_toolbar(&new_tab_btn);
    tab_manager.wire_keyboard(&window);
    tab_manager.wire_app_menu(profile.web_context.clone());
    tab_manager.wire_history(profile.history.clone());
    tab_manager.set_new_window_handler(make_spawner(app, windows, shared));

    windows.borrow_mut().push(tab_manager.clone());
    let windows_for_destroy = windows.clone();
    window.connect_destroy(move |window| {
        windows_for_destroy
            .borrow_mut()
            .retain(|manager| !manager.owns_window(window));
    });

    tab_manager.open_initial_home();
    window.show_all();
    Some(tab_manager)
}

/// Creates (once) and returns the shared WebKit context and history store.
fn ensure_profile(app: &Application, shared: &SharedProfileSlot) -> Option<Rc<SharedProfile>> {
    let existing = shared.borrow().clone();
    if let Some(profile) = existing {
        return Some(profile);
    }
    let Some(data_dir) = app_data_dir() else {
        // Never fall back to a directory another user could control (V-4).
        show_fatal_profile_error(
            app,
            "no private profile directory is available for this user",
        );
        return None;
    };
    theme::load_preference(&data_dir);
    let web_context = match persistent_web_context(&data_dir) {
        Ok(ctx) => ctx,
        Err(err) => {
            show_fatal_profile_error(app, &err.to_string());
            return None;
        }
    };
    downloads::wire(&web_context);
    let history = Rc::new(RefCell::new(HistoryStore::load(
        data_dir.join("history.json"),
    )));
    let profile = Rc::new(SharedProfile {
        web_context,
        history,
    });
    *shared.borrow_mut() = Some(profile.clone());
    Some(profile)
}

/// The Ctrl+N / menu callback: build another window over the same profile.
fn make_spawner(
    app: &Application,
    windows: &WindowList,
    shared: &SharedProfileSlot,
) -> Rc<dyn Fn()> {
    let app = app.clone();
    let windows = Rc::downgrade(windows);
    let shared = Rc::downgrade(shared);
    Rc::new(move || {
        let (Some(windows), Some(shared)) = (windows.upgrade(), shared.upgrade()) else {
            return;
        };
        if let Some(manager) = build_window(&app, &windows, &shared) {
            manager.present();
        }
    })
}

/// GApplication turns every non-URI argument into a file relative to the
/// working directory, so `nodaysidle-browser wikipedia.org` used to open
/// file:///<cwd>/wikipedia.org (R-10). Resolve arguments here instead: an
/// existing file opens as a file, and anything else goes through the same
/// resolution as the address bar.
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

fn show_fatal_profile_error(app: &Application, message: &str) {
    eprintln!("nodaysidle-browser: {message}");
    let dialog = gtk::MessageDialog::new(
        None::<&gtk::Window>,
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Close,
        "Could not open a private profile",
    );
    dialog.set_secondary_text(Some(message));
    if let Some(window) = app.active_window() {
        dialog.set_transient_for(Some(&window));
    }
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
    dialog.show_all();
    dialog.run();
    app.quit();
}

#[cfg(test)]
mod tests {
    use super::{command_line_options, should_confirm_quit};
    use super::{command_line_target, CommandLineOptions};
    use std::path::Path;

    fn target(arg: &str, existing: &[&str]) -> Option<String> {
        command_line_target(arg, Some(Path::new("/work")), &|path: &Path| {
            existing.iter().any(|existing| path == Path::new(existing))
        })
    }

    fn options(args: &[&str]) -> CommandLineOptions {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        command_line_options(&args, Some(Path::new("/work")), &|_| false)
    }

    #[test]
    fn bare_host_arguments_resolve_like_the_address_bar() {
        assert_eq!(
            target("wikipedia.org", &[]),
            Some("https://wikipedia.org".into())
        );
        assert_eq!(
            target("localhost:3000", &[]),
            Some("http://localhost:3000".into())
        );
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
        assert_eq!(
            target("./missing.html", &[]),
            Some("file:///work/missing.html".into())
        );
        assert_eq!(
            target("/tmp/page.html", &[]),
            Some("file:///tmp/page.html".into())
        );
    }

    #[test]
    fn new_window_flags_are_recognized_and_stripped_from_targets() {
        assert_eq!(
            options(&["prog", "--new-window", "wikipedia.org"]),
            CommandLineOptions {
                new_window: true,
                targets: vec!["https://wikipedia.org".into()],
            }
        );
        assert_eq!(
            options(&["prog", "-n"]),
            CommandLineOptions {
                new_window: true,
                targets: Vec::new(),
            }
        );
        assert_eq!(
            options(&["prog"]),
            CommandLineOptions {
                new_window: false,
                targets: Vec::new(),
            }
        );
    }

    #[test]
    fn options_and_program_name_are_left_alone() {
        // `--` is consumed by GApplication before the handler sees the args, so
        // the flag is only checked for recognition; unknown options are ignored.
        assert_eq!(
            options(&["prog", "--gapplication-service", "wikipedia.org", "-x"]),
            CommandLineOptions {
                new_window: false,
                targets: vec!["https://wikipedia.org".into()],
            }
        );
    }

    #[test]
    fn only_the_last_window_confirms_quit_for_downloads() {
        assert!(should_confirm_quit(true, true));
        assert!(!should_confirm_quit(false, true));
        assert!(!should_confirm_quit(true, false));
        assert!(!should_confirm_quit(false, false));
    }
}
