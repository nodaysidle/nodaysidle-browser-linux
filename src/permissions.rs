use gtk::prelude::*;
use webkit2gtk::{
    GeolocationPermissionRequest, NotificationPermissionRequest, PermissionRequest,
    PermissionRequestExt, PointerLockPermissionRequest, UserMediaPermissionRequest,
    UserMediaPermissionRequestExt, WebView, WebViewExt,
};

pub fn wire(web_view: &WebView) {
    web_view.connect_permission_request(handle_request);
}

fn handle_request(view: &WebView, request: &PermissionRequest) -> bool {
    let Some(permission) = permission_name(request) else {
        request.deny();
        return true;
    };
    let Some(parent) = view
        .toplevel()
        .and_then(|widget| widget.downcast::<gtk::Window>().ok())
    else {
        request.deny();
        return true;
    };

    let origin = view
        .uri()
        .map(|uri| permission_origin(&uri))
        .unwrap_or_else(|| "this page".to_string());
    let dialog = gtk::Dialog::with_buttons(
        Some("Site permission request"),
        Some(&parent),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        &[
            ("_Deny", gtk::ResponseType::Reject),
            ("_Allow", gtk::ResponseType::Accept),
        ],
    );
    dialog.set_default_response(gtk::ResponseType::Reject);

    let label = gtk::Label::new(None);
    label.set_text(&format!(
        "{origin} requests {permission}. Allow this request?"
    ));
    label.set_line_wrap(true);
    label.set_max_width_chars(56);
    label.set_margin_top(16);
    label.set_margin_bottom(16);
    label.set_margin_start(16);
    label.set_margin_end(16);
    dialog.content_area().pack_start(&label, true, true, 0);
    dialog.show_all();
    let response = dialog.run();
    dialog.close();

    if permission_response_allows(response) {
        request.allow();
    } else {
        request.deny();
    }
    true
}

fn permission_name(request: &PermissionRequest) -> Option<&'static str> {
    if request.is::<GeolocationPermissionRequest>() {
        Some("your location")
    } else if let Some(media) = request.downcast_ref::<UserMediaPermissionRequest>() {
        Some(
            match (media.is_for_video_device(), media.is_for_audio_device()) {
                (true, true) => "your camera and microphone",
                (true, false) => "your camera",
                (false, true) => "your microphone",
                (false, false) => "media devices",
            },
        )
    } else if request.is::<NotificationPermissionRequest>() {
        Some("notifications")
    } else if request.is::<PointerLockPermissionRequest>() {
        Some("pointer lock")
    } else {
        None
    }
}

fn permission_origin(uri: &str) -> String {
    let Ok(parsed) = url::Url::parse(uri) else {
        return "this page".to_string();
    };
    match parsed.origin().ascii_serialization().as_str() {
        "null" if parsed.scheme() == "file" => "this local file".to_string(),
        "null" => "this page".to_string(),
        origin => origin.to_string(),
    }
}

fn permission_response_allows(response: gtk::ResponseType) -> bool {
    response == gtk::ResponseType::Accept
}

#[cfg(test)]
mod tests {
    use super::{permission_origin, permission_response_allows};

    #[test]
    fn permission_prompt_shows_only_the_origin() {
        assert_eq!(
            permission_origin("https://example.com/path?token=secret#section"),
            "https://example.com"
        );
        assert_eq!(
            permission_origin("file:///tmp/page.html"),
            "this local file"
        );
    }

    #[test]
    fn a_permission_is_allowed_only_after_explicit_acceptance() {
        assert!(permission_response_allows(gtk::ResponseType::Accept));
        assert!(!permission_response_allows(gtk::ResponseType::Reject));
        assert!(!permission_response_allows(gtk::ResponseType::DeleteEvent));
    }
}
