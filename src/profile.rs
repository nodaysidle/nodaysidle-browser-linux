use std::path::PathBuf;
use webkit2gtk::{WebContext, WebsiteDataManager};

/// Shared WebKit profile: cookies and site storage persist across launches.
pub fn persistent_web_context(data_root: &PathBuf) -> WebContext {
    let data_dir = data_root.join("webkit-data");
    let cache_dir = data_root.join("webkit-cache");

    let _ = std::fs::create_dir_all(&data_dir);
    let _ = std::fs::create_dir_all(&cache_dir);

    let manager = WebsiteDataManager::builder()
        .base_data_directory(data_dir.to_string_lossy().as_ref())
        .base_cache_directory(cache_dir.to_string_lossy().as_ref())
        .build();

    WebContext::with_website_data_manager(&manager)
}

pub fn app_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join("nodaysidle-browser")
}
