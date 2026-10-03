use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use webkit2gtk::{
    CookieManagerExt, CookiePersistentStorage, WebContext, WebContextExt, WebsiteDataManager,
};

/// Shared WebKit profile: cookies and site storage persist across launches.
pub fn persistent_web_context(data_root: &Path) -> WebContext {
    let data_dir = data_root.join("webkit-data");
    let cache_dir = data_root.join("webkit-cache");

    let _ = std::fs::create_dir_all(&data_dir);
    let _ = std::fs::create_dir_all(&cache_dir);

    let manager = WebsiteDataManager::builder()
        .base_data_directory(data_dir.to_string_lossy().as_ref())
        .base_cache_directory(cache_dir.to_string_lossy().as_ref())
        .build();

    let web_context = WebContext::with_website_data_manager(&manager);
    web_context.set_sandbox_enabled(true);
    let cookie_path = data_dir.join("cookies.sqlite");
    if let Err(err) = ensure_private_cookie_file(&cookie_path) {
        eprintln!("Could not prepare persistent cookie storage: {err}");
    }
    if let Some(cookie_manager) = web_context.cookie_manager() {
        cookie_manager.set_persistent_storage(
            &cookie_path.to_string_lossy(),
            CookiePersistentStorage::Sqlite,
        );
    }
    web_context
}

#[cfg(unix)]
fn ensure_private_cookie_file(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
}

pub fn app_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join("nodaysidle-browser")
}

#[cfg(all(test, unix))]
mod tests {
    use super::{ensure_private_cookie_file, persistent_web_context};
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use webkit2gtk::WebContextExt;

    #[test]
    fn shared_web_context_enables_the_web_process_sandbox() {
        // WebKit creates GTK widgets while building a WebContext; without an
        // initialised display, GTK dereferences NULL settings and segfaults.
        if gtk::init().is_err() {
            eprintln!("skipping: no display available for GTK");
            return;
        }
        let dir = std::env::temp_dir().join(format!(
            "nodaysidle-browser-sandbox-test-{}",
            std::process::id()
        ));
        let web_context = persistent_web_context(&dir);
        assert!(web_context.is_sandbox_enabled());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cookie_file_permissions_are_restricted_to_the_current_user() {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "nodaysidle-browser-cookie-test-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let cookie_file = dir.join("cookies.sqlite");
        std::fs::write(&cookie_file, b"existing cookie database").unwrap();

        ensure_private_cookie_file(&cookie_file).unwrap();

        let permissions = std::fs::metadata(&cookie_file).unwrap().permissions();
        assert_eq!(permissions.mode() & 0o777, 0o600);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
