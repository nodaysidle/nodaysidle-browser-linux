use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use webkit2gtk::{
    CookieManagerExt, CookiePersistentStorage, WebContext, WebContextExt, WebsiteDataManager,
};

/// Shared WebKit profile: cookies and site storage persist across launches.
pub fn persistent_web_context(data_root: &Path) -> WebContext {
    let data_dir = data_root.join("webkit-data");
    let cache_dir = data_root.join("webkit-cache");

    // The profile holds cookies, site storage and history: keep it private to
    // the user, and say so when it cannot be created (X-30).
    for dir in [data_root, data_dir.as_path(), cache_dir.as_path()] {
        if let Err(err) = ensure_private_dir(dir) {
            eprintln!("Could not prepare profile directory {}: {err}", dir.display());
        }
    }

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

/// Creates `path` (and missing parents) and restricts the directory itself to
/// the current user. Parents that already exist are left alone.
#[cfg(unix)]
pub fn ensure_private_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

/// Absolute profile directory. Falls back to ~/.local/share and then to the
/// temporary directory instead of a path relative to the working directory.
pub fn app_data_dir() -> PathBuf {
    data_dir_from(dirs::data_local_dir(), dirs::home_dir(), std::env::temp_dir())
}

fn data_dir_from(data_local: Option<PathBuf>, home: Option<PathBuf>, temp: PathBuf) -> PathBuf {
    let base = data_local
        .filter(|dir| dir.is_absolute())
        .or_else(|| {
            home.filter(|dir| dir.is_absolute())
                .map(|home| home.join(".local/share"))
        });
    match base {
        Some(base) => base.join("nodaysidle-browser"),
        None => temp.join(format!(
            "nodaysidle-browser-{}",
            std::env::var("USER").unwrap_or_else(|_| "user".into())
        )),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::{
        data_dir_from, ensure_private_cookie_file, ensure_private_dir, persistent_web_context,
    };
    use std::path::PathBuf;
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
    fn profile_directory_is_always_absolute() {
        let temp = PathBuf::from("/tmp");
        assert_eq!(
            data_dir_from(Some("/home/me/.local/share".into()), None, temp.clone()),
            PathBuf::from("/home/me/.local/share/nodaysidle-browser")
        );
        assert_eq!(
            data_dir_from(None, Some("/home/me".into()), temp.clone()),
            PathBuf::from("/home/me/.local/share/nodaysidle-browser")
        );
        assert_eq!(
            data_dir_from(Some("relative".into()), Some("also-relative".into()), temp.clone())
                .parent(),
            Some(temp.as_path())
        );
    }

    #[test]
    fn profile_directories_are_private_and_creation_errors_are_reported() {
        let dir = std::env::temp_dir().join(format!(
            "nodaysidle-browser-dir-test-{}",
            std::process::id()
        ));
        let nested = dir.join("a/b");
        ensure_private_dir(&nested).unwrap();
        let mode = std::fs::metadata(&nested).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);

        // An existing directory that is too open gets tightened.
        std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o755)).unwrap();
        ensure_private_dir(&nested).unwrap();
        let mode = std::fs::metadata(&nested).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);

        // A regular file in the way is an error, not a silent success.
        let file = dir.join("file");
        std::fs::write(&file, b"x").unwrap();
        assert!(ensure_private_dir(&file.join("sub")).is_err());
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
