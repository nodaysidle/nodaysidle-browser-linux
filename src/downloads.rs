use gio::prelude::*;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone)]
struct ActiveDownload {
    id: usize,
    filename: String,
    state: Rc<Cell<DownloadState>>,
    download: Download,
}

thread_local! {
    static ACTIVE_DOWNLOADS: RefCell<Vec<ActiveDownload>> = const { RefCell::new(Vec::new()) };
    static NEXT_DOWNLOAD_ID: Cell<usize> = const { Cell::new(1) };
}

fn unregister_download(id: usize) {
    ACTIVE_DOWNLOADS.with(|d| {
        d.borrow_mut().retain(|item| item.id != id);
    });
}

pub fn has_active_downloads() -> bool {
    active_download_count() > 0
}

pub fn active_download_count() -> usize {
    ACTIVE_DOWNLOADS.with(|d| {
        d.borrow()
            .iter()
            .filter(|item| item.state.get() == DownloadState::Active)
            .count()
    })
}

pub fn cancel_all_active() {
    let list = ACTIVE_DOWNLOADS.with(|d| d.borrow().clone());
    for item in list {
        request_cancel(&item.download, &item.state);
    }
}

pub fn confirm_quit(parent: &gtk::Window) -> bool {
    let (count, filename) = ACTIVE_DOWNLOADS.with(|d| {
        let borrowed = d.borrow();
        let active: Vec<_> = borrowed
            .iter()
            .filter(|item| item.state.get() == DownloadState::Active)
            .collect();
        (
            active.len(),
            active.first().map(|item| item.filename.clone()),
        )
    });
    if count == 0 {
        return true;
    }
    let (primary, secondary) = quit_confirmation_text(count, filename.as_deref());
    let dialog = gtk::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk::MessageType::Question)
        .buttons(gtk::ButtonsType::None)
        .text(&primary)
        .secondary_text(&secondary)
        .build();
    dialog.add_button("_Continue downloading", gtk::ResponseType::No);
    dialog.add_button("_Cancel download and quit", gtk::ResponseType::Yes);
    dialog.set_default_response(gtk::ResponseType::No);
    let confirmed = dialog.run() == gtk::ResponseType::Yes;
    dialog.close();
    confirmed
}

pub(crate) fn quit_confirmation_text(
    count: usize,
    first_filename: Option<&str>,
) -> (String, String) {
    if count <= 1 {
        let name = first_filename.unwrap_or("file");
        (
            format!("Cancel download of “{name}” and quit?"),
            "Closing the window will cancel the download in progress.".to_string(),
        )
    } else {
        (
            format!("Cancel {count} downloads and quit?"),
            "Closing the window will cancel all downloads in progress.".to_string(),
        )
    }
}
use webkit2gtk::{Download, DownloadExt, WebContext, WebContextExt};

pub fn wire(web_context: &WebContext) {
    web_context.connect_download_started(|_, download| {
        download.connect_decide_destination(|download, suggested_filename| {
            let filename = safe_suggested_filename(suggested_filename);
            let Some(parent) = browser_window_for(download) else {
                download.cancel();
                return true;
            };

            let Some(path) = choose_destination(&parent, &filename) else {
                download.cancel();
                return true;
            };

            let destination = gio::File::for_path(path).uri();
            download.set_allow_overwrite(true);
            download.set_destination(destination.as_str());
            show_progress(download, &parent, &filename);
            true
        });
    });
}

/// The main browser window to parent download dialogs to. A download can
/// start in a pop-up window (X-3) that closes itself right after, often via
/// window.close(); a progress dialog parented to it with DESTROY_WITH_PARENT
/// would be destroyed and cancel the download without asking (V-2). Pop-ups
/// are transient for the browser window, so follow that chain to the top;
/// fall back to the application's windows if the view is already gone.
fn browser_window_for(download: &Download) -> Option<gtk::Window> {
    let from_view = download
        .web_view()
        .and_then(|view| view.toplevel())
        .and_then(|widget| widget.downcast::<gtk::Window>().ok())
        .filter(|window| window.is_toplevel());
    if let Some(mut window) = from_view {
        while let Some(parent) = window.transient_for() {
            window = parent;
        }
        return Some(window);
    }
    let app = gio::Application::default()?
        .downcast::<gtk::Application>()
        .ok()?;
    app.active_window()
        .or_else(|| app.windows().into_iter().next())
}

fn choose_destination(parent: &gtk::Window, filename: &str) -> Option<PathBuf> {
    let dialog = gtk::FileChooserDialog::with_buttons(
        Some("Save download"),
        Some(parent),
        gtk::FileChooserAction::Save,
        &[
            ("_Cancel", gtk::ResponseType::Cancel),
            ("_Save", gtk::ResponseType::Accept),
        ],
    );
    dialog.set_current_name(filename);
    dialog.set_do_overwrite_confirmation(true);
    if let Some(folder) = default_download_dir() {
        if !dialog.set_current_folder(&folder) {
            if let Some(home) = dirs::home_dir() {
                let _ = dialog.set_current_folder(home);
            }
        }
    }

    let path = if dialog.run() == gtk::ResponseType::Accept {
        dialog.filename()
    } else {
        None
    };
    dialog.close();
    path
}

fn default_download_dir() -> Option<PathBuf> {
    dirs::download_dir()
        .filter(|path| path.is_dir())
        .or_else(|| dirs::home_dir().filter(|path| path.is_dir()))
}

fn show_progress(download: &Download, parent: &gtk::Window, filename: &str) {
    let dialog = gtk::Dialog::with_buttons(
        Some("Downloading"),
        Some(parent),
        gtk::DialogFlags::DESTROY_WITH_PARENT,
        &[],
    );
    dialog.set_modal(false);

    let content = dialog.content_area();
    let filename_label = gtk::Label::new(Some(filename));
    filename_label.set_xalign(0.0);
    filename_label.set_line_wrap(true);
    filename_label.set_margin_top(12);
    filename_label.set_margin_start(12);
    filename_label.set_margin_end(12);
    content.pack_start(&filename_label, false, false, 0);

    let progress = gtk::ProgressBar::new();
    progress.set_show_text(true);
    progress.set_margin_top(8);
    progress.set_margin_start(12);
    progress.set_margin_end(12);
    content.pack_start(&progress, false, false, 0);

    let status = gtk::Label::new(Some("Starting download…"));
    status.set_xalign(0.0);
    status.set_margin_top(6);
    status.set_margin_bottom(8);
    status.set_margin_start(12);
    status.set_margin_end(12);
    content.pack_start(&status, false, false, 0);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.set_halign(gtk::Align::End);
    buttons.set_margin_bottom(10);
    buttons.set_margin_end(12);
    let cancel_button = gtk::Button::with_label("Cancel");
    let close_button = gtk::Button::with_label("Close");
    close_button.set_no_show_all(true);
    buttons.pack_start(&cancel_button, false, false, 0);
    buttons.pack_start(&close_button, false, false, 0);
    content.pack_start(&buttons, false, false, 0);

    // One shared state for every path that can stop the download. WebKit's
    // cancel is asynchronous (the failed/finished signals arrive later over
    // IPC), so the old code cancelled twice when Cancel closed the dialog and
    // the destroy handler cancelled again; the second cancel crashed (N-1).
    let state = Rc::new(Cell::new(DownloadState::Active));

    let download_id = NEXT_DOWNLOAD_ID.with(|id| {
        let next = id.get();
        id.set(next + 1);
        next
    });
    ACTIVE_DOWNLOADS.with(|d| {
        d.borrow_mut().push(ActiveDownload {
            id: download_id,
            filename: filename.to_string(),
            state: state.clone(),
            download: download.clone(),
        });
    });
    // Closing the window with the window manager while the download runs asks
    // first instead of silently cancelling (R-10b).
    let state_on_delete = state.clone();
    let download_on_delete = download.clone();
    let filename_on_delete = filename.to_string();
    dialog.connect_delete_event(move |dialog, _| {
        if state_on_delete.get() != DownloadState::Active {
            return glib::Propagation::Proceed;
        }
        if confirm_cancel(dialog.upcast_ref(), &filename_on_delete) {
            request_cancel(&download_on_delete, &state_on_delete);
            glib::Propagation::Proceed
        } else {
            glib::Propagation::Stop
        }
    });

    // The dialog is also destroyed with its parent, the main browser window,
    // when the browser quits; an unfinished download cannot continue without
    // the app, so stop it once.
    let state_on_destroy = state.clone();
    let download_on_destroy = download.clone();
    dialog.connect_destroy(move |_| {
        unregister_download(download_id);
        request_cancel(&download_on_destroy, &state_on_destroy);
    });
    // Cancel keeps the window open: WebKit confirms asynchronously with
    // `failed` (CancelledByUser), which shows "Download cancelled" and a Close
    // button (V-10). If the transfer completed before the cancel arrived,
    // `finished` reports it as complete instead.
    let state_on_cancel = state.clone();
    let download_on_cancel = download.clone();
    let status_weak = status.downgrade();
    cancel_button.connect_clicked(move |button| {
        unregister_download(download_id);
        request_cancel(&download_on_cancel, &state_on_cancel);
        button.set_sensitive(false);
        if let Some(status) = status_weak.upgrade() {
            status.set_text("Cancelling…");
        }
    });

    let dialog_weak = dialog.downgrade();
    close_button.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    });

    let progress_weak = progress.downgrade();
    let status_weak = status.downgrade();
    let state_on_progress = state.clone();
    download.connect_notify_local(Some("estimated-progress"), move |download, _| {
        if state_on_progress.get() != DownloadState::Active {
            return;
        }
        if let Some(progress) = progress_weak.upgrade() {
            let fraction = download.estimated_progress().clamp(0.0, 1.0);
            progress.set_fraction(fraction);
            progress.set_text(Some(&format!("{:.0}%", fraction * 100.0)));
        }
        if let Some(status) = status_weak.upgrade() {
            status.set_text("Downloading… (closing this window asks before cancelling)");
        }
    });

    let progress_weak = progress.downgrade();
    let status_weak = status.downgrade();
    let cancel_weak = cancel_button.downgrade();
    let close_weak = close_button.downgrade();
    let state_on_finish = state.clone();
    download.connect_finished(move |_| {
        unregister_download(download_id);
        let failed = state_on_finish.get() == DownloadState::Failed;
        state_on_finish.set(state_on_finish.get().finish());
        if failed {
            return;
        }
        if let Some(progress) = progress_weak.upgrade() {
            progress.set_fraction(1.0);
            progress.set_text(Some("100%"));
        }
        if let Some(status) = status_weak.upgrade() {
            status.set_text("Download complete");
        }
        if let Some(cancel) = cancel_weak.upgrade() {
            cancel.hide();
        }
        if let Some(close) = close_weak.upgrade() {
            close.show();
        }
    });

    let status_weak = status.downgrade();
    let cancel_weak = cancel_button.downgrade();
    let close_weak = close_button.downgrade();
    let state_on_failure = state;
    download.connect_failed(move |_, error| {
        unregister_download(download_id);
        state_on_failure.set(DownloadState::Failed);
        if let Some(status) = status_weak.upgrade() {
            if error.matches(webkit2gtk::DownloadError::CancelledByUser) {
                status.set_text("Download cancelled");
            } else {
                status.set_text(&format!("Download failed: {}", error.message()));
            }
        }
        if let Some(cancel) = cancel_weak.upgrade() {
            cancel.hide();
        }
        if let Some(close) = close_weak.upgrade() {
            close.show();
        }
    });

    dialog.show_all();
    close_button.hide();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DownloadState {
    Active,
    /// `webkit_download_cancel` was called; WebKit has not reported back yet.
    Cancelling,
    Failed,
    Finished,
}

impl DownloadState {
    /// Returns the next state and whether `cancel()` must be called now.
    fn cancel(self) -> (Self, bool) {
        match self {
            DownloadState::Active => (DownloadState::Cancelling, true),
            other => (other, false),
        }
    }

    fn finish(self) -> Self {
        match self {
            DownloadState::Failed => DownloadState::Failed,
            _ => DownloadState::Finished,
        }
    }
}

/// Cancels at most once, and never after WebKit reported the end of the
/// download.
fn request_cancel(download: &Download, state: &Cell<DownloadState>) {
    let (next, call_cancel) = state.get().cancel();
    state.set(next);
    if call_cancel {
        download.cancel();
    }
}

fn confirm_cancel(parent: &gtk::Window, filename: &str) -> bool {
    let dialog = gtk::MessageDialog::new(
        Some(parent),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        gtk::MessageType::Question,
        gtk::ButtonsType::None,
        "Cancel this download?",
    );
    dialog.set_secondary_text(Some(&format!(
        "Closing this window stops the download of \u{201c}{filename}\u{201d}."
    )));
    dialog.add_button("_Keep Downloading", gtk::ResponseType::Reject);
    dialog.add_button("_Cancel Download", gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Reject);
    let response = dialog.run();
    dialog.close();
    response == gtk::ResponseType::Accept
}

fn safe_suggested_filename(suggested: &str) -> String {
    let basename = suggested.rsplit(['/', '\\']).next().unwrap_or_default();
    let filename = basename
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();
    let filename = filename.trim();
    if filename.is_empty() || filename == "." || filename == ".." {
        "download".to_string()
    } else {
        filename.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{quit_confirmation_text, safe_suggested_filename, DownloadState};

    #[test]
    fn quit_confirmation_wording_distinguishes_single_and_multiple_downloads() {
        let (single, sub) = quit_confirmation_text(1, Some("archive.tar.gz"));
        assert!(single.contains("archive.tar.gz"));
        assert!(sub.contains("cancel the download"));

        let (multiple, sub_mult) = quit_confirmation_text(3, None);
        assert!(multiple.contains("3 downloads"));
        assert!(sub_mult.contains("all downloads"));
    }

    #[test]
    fn a_download_is_cancelled_at_most_once() {
        let (state, call) = DownloadState::Active.cancel();
        assert_eq!(state, DownloadState::Cancelling);
        assert!(call);
        // Cancel button followed by the dialog's destroy handler (N-1).
        let (state, call) = state.cancel();
        assert_eq!(state, DownloadState::Cancelling);
        assert!(!call);
    }

    #[test]
    fn finished_or_failed_downloads_are_never_cancelled() {
        assert!(!DownloadState::Finished.cancel().1);
        assert!(!DownloadState::Failed.cancel().1);
        assert_eq!(DownloadState::Failed.finish(), DownloadState::Failed);
        assert_eq!(DownloadState::Active.finish(), DownloadState::Finished);
        assert_eq!(DownloadState::Cancelling.finish(), DownloadState::Finished);
    }

    #[test]
    fn suggested_download_names_cannot_escape_the_chosen_directory() {
        assert_eq!(safe_suggested_filename("../../etc/passwd"), "passwd");
        assert_eq!(safe_suggested_filename(r"..\..\secret.txt"), "secret.txt");
    }

    #[test]
    fn empty_and_control_only_names_get_a_safe_fallback() {
        assert_eq!(safe_suggested_filename("../"), "download");
        assert_eq!(safe_suggested_filename("\n\t"), "download");
    }
}
