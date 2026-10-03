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
    match xdg_data_dir(dirs::data_local_dir(), dirs::home_dir()) {
        Some(dir) => dir,
        None => {
            let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
            temp_data_dir(&std::env::temp_dir(), &user, current_uid())
        }
    }
}

fn xdg_data_dir(data_local: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    data_local
        .filter(|dir| dir.is_absolute())
        .or_else(|| {
            home.filter(|dir| dir.is_absolute())
                .map(|home| home.join(".local/share"))
        })
        .map(|base| base.join("nodaysidle-browser"))
}

/// The effective user id, read from the owner of /proc/self (Linux) so no
/// unsafe FFI call is needed. `None` if /proc is not available.
#[cfg(unix)]
fn current_uid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").ok().map(|meta| meta.uid())
}

/// Profile in the shared temporary directory, used only without a usable
/// XDG data dir or HOME. The predictable `nodaysidle-browser-$USER` name is
/// used only if it is a real directory owned by us and private after chmod;
/// otherwise another local user could have planted it (V-4), so a fresh
/// private directory is created instead (not persistent across launches).
fn temp_data_dir(temp: &Path, user: &str, uid: Option<u32>) -> PathBuf {
    let preferred = temp.join(format!("nodaysidle-browser-{user}"));
    let refusal = match uid {
        Some(uid) => match claim_private_dir(&preferred, uid) {
            Ok(()) => return preferred,
            Err(err) => err.to_string(),
        },
        None => "cannot determine the current user id".to_string(),
    };
    for attempt in 0..100u32 {
        let fresh = temp.join(format!(
            "nodaysidle-browser-{user}-{}-{attempt}",
            std::process::id()
        ));
        if create_new_private_dir(&fresh).is_ok() {
            eprintln!(
                "Not using profile directory {} ({refusal}); using {} for this session only",
                preferred.display(),
                fresh.display()
            );
            return fresh;
        }
    }
    eprintln!("Could not create a private profile directory in {}", temp.display());
    preferred
}

/// Creates `path` as a 0700 directory, failing if anything already exists.
#[cfg(unix)]
fn create_new_private_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new().mode(0o700).create(path)
}

/// Makes sure `path` is a directory (not a symlink) owned by `uid` with mode
/// 0700, creating it if missing. Errors explain why it must not be used.
#[cfg(unix)]
fn claim_private_dir(path: &Path, uid: u32) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    match create_new_private_dir(path) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
        Err(err) => return Err(err),
    }
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.file_type().is_dir() {
        return Err(Error::other("it is not a directory"));
    }
    if meta.uid() != uid {
        return Err(Error::other(format!("it is owned by uid {}", meta.uid())));
    }
    if meta.mode() & 0o777 != 0o700 {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
        let mode = std::fs::symlink_metadata(path)?.mode() & 0o777;
        if mode != 0o700 {
            return Err(Error::other(format!("its mode is {mode:o}, not 700")));
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::{claim_private_dir, ensure_private_cookie_file, ensure_private_dir};
    use super::{temp_data_dir, xdg_data_dir};
    use std::path::PathBuf;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn profile_directory_is_always_absolute() {
        assert_eq!(
            xdg_data_dir(Some("/home/me/.local/share".into()), None),
            Some(PathBuf::from("/home/me/.local/share/nodaysidle-browser"))
        );
        assert_eq!(
            xdg_data_dir(None, Some("/home/me".into())),
            Some(PathBuf::from("/home/me/.local/share/nodaysidle-browser"))
        );
        assert_eq!(xdg_data_dir(Some("relative".into()), Some("also-relative".into())), None);
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nodaysidle-browser-{name}-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        dir
    }

    #[test]
    fn the_temp_fallback_uses_its_own_private_directory() {
        let temp = scratch("tempfb");
        let uid = super::current_uid().expect("/proc/self is readable");
        let dir = temp_data_dir(&temp, "me", Some(uid));
        assert_eq!(dir, temp.join("nodaysidle-browser-me"));
        assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);

        // An existing directory of ours that is too open is tightened and kept.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(temp_data_dir(&temp, "me", Some(uid)), dir);
        assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
        std::fs::remove_dir_all(&temp).unwrap();
    }

    #[test]
    fn the_temp_fallback_refuses_directories_it_cannot_trust() {
        let temp = scratch("tempdeny");
        let uid = super::current_uid().expect("/proc/self is readable");
        let preferred = temp.join("nodaysidle-browser-me");
        std::fs::create_dir(&preferred).unwrap();

        // Owned by someone else (simulated by claiming to be another uid).
        assert!(claim_private_dir(&preferred, uid + 1).is_err());
        let other = temp_data_dir(&temp, "me", Some(uid + 1));
        assert_ne!(other, preferred);
        assert!(other.starts_with(&temp));
        assert_eq!(std::fs::metadata(&other).unwrap().permissions().mode() & 0o777, 0o700);

        // A symlink planted at the predictable name is not followed.
        std::fs::remove_dir(&preferred).unwrap();
        let target = temp.join("elsewhere");
        std::fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, &preferred).unwrap();
        assert!(claim_private_dir(&preferred, uid).is_err());
        assert_ne!(temp_data_dir(&temp, "me", Some(uid)), preferred);

        // Without a known uid nothing predictable is trusted.
        std::fs::remove_file(&preferred).unwrap();
        assert_ne!(temp_data_dir(&temp, "me", None), preferred);
        std::fs::remove_dir_all(&temp).unwrap();
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
