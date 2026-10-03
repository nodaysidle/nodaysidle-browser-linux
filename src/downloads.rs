use gio::prelude::*;
use gtk::prelude::*;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use webkit2gtk::{Download, DownloadExt, WebContext, WebContextExt};

pub fn wire(web_context: &WebContext) {
    web_context.connect_download_started(|_, download| {
        download.connect_decide_destination(|download, suggested_filename| {
            let filename = safe_suggested_filename(suggested_filename);
            let Some(parent) = download
                .web_view()
                .and_then(|view| view.toplevel())
                .and_then(|widget| widget.downcast::<gtk::Window>().ok())
            else {
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

    let completed = Rc::new(Cell::new(false));
    let completed_on_destroy = completed.clone();
    let download_on_destroy = download.clone();
    dialog.connect_destroy(move |_| {
        if !completed_on_destroy.get() {
            download_on_destroy.cancel();
        }
    });

    let completed_on_cancel = completed.clone();
    let download_on_cancel = download.clone();
    let dialog_weak = dialog.downgrade();
    cancel_button.connect_clicked(move |_| {
        if !completed_on_cancel.get() {
            download_on_cancel.cancel();
        }
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    });

    let dialog_weak = dialog.downgrade();
    close_button.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    });

    let progress_weak = progress.downgrade();
    download.connect_notify_local(Some("estimated-progress"), move |download, _| {
        if let Some(progress) = progress_weak.upgrade() {
            let fraction = download.estimated_progress().clamp(0.0, 1.0);
            progress.set_fraction(fraction);
            progress.set_text(Some(&format!("{:.0}%", fraction * 100.0)));
        }
    });

    let progress_weak = progress.downgrade();
    let status_weak = status.downgrade();
    let cancel_weak = cancel_button.downgrade();
    let close_weak = close_button.downgrade();
    let completed_on_finish = completed.clone();
    download.connect_finished(move |_| {
        completed_on_finish.set(true);
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
    let completed_on_failure = completed;
    download.connect_failed(move |_, _| {
        completed_on_failure.set(true);
        if let Some(status) = status_weak.upgrade() {
            status.set_text("Download failed");
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
    use super::safe_suggested_filename;

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
