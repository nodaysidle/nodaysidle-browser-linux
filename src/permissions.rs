use gtk::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use webkit2gtk::{
    GeolocationPermissionRequest, NotificationPermissionRequest, PermissionRequest,
    PermissionRequestExt, PointerLockPermissionRequest, UserMediaPermissionRequest,
    UserMediaPermissionRequestExt, WebView, WebViewExt,
};

thread_local! {
    /// Deny answers per (origin, permission) for this session only. Allows are
    /// never cached because WebKitGTK cannot attribute the requesting frame.
    static DECISIONS: RefCell<SessionDecisions> = RefCell::new(SessionDecisions::default());
}

#[derive(Default)]
struct SessionDecisions {
    denials: HashMap<(String, &'static str), ()>,
}

impl SessionDecisions {
    fn is_denied(&self, origin: &str, permission: &'static str) -> bool {
        self.denials.contains_key(&(origin.to_string(), permission))
    }

    fn remember_denial(&mut self, origin: &str, permission: &'static str) {
        if origin.contains("://") {
            self.denials.insert((origin.to_string(), permission), ());
        }
    }
}

pub fn clear_session_grants() {
    DECISIONS.with(|decisions| decisions.borrow_mut().denials.clear());
}

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

    // WebKitGTK 4.1 does not say which frame asked (WebKitPermissionRequest
    // carries no origin), so the prompt names the page's origin and says that
    // an embedded frame may be the requester.
    let origin = view
        .uri()
        .map(|uri| permission_origin(&uri))
        .unwrap_or_else(|| "this page".to_string());
    if DECISIONS.with(|decisions| decisions.borrow().is_denied(&origin, permission)) {
        request.deny();
        return true;
    }
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
    label.set_text(&prompt_text(&origin, permission));
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

    let allowed = permission_response_allows(response);
    let still_valid = view
        .uri()
        .map(|uri| permission_origin(&uri) == origin)
        .unwrap_or(false);
    if !still_valid {
        request.deny();
        return true;
    }
    if matches!(response, gtk::ResponseType::Reject) {
        DECISIONS.with(|decisions| {
            decisions.borrow_mut().remember_denial(&origin, permission);
        });
        request.deny();
    } else if allowed {
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

fn prompt_text(origin: &str, permission: &str) -> String {
    format!(
        "{origin} (or a site embedded in it) requests {permission}. Allow this request?\n\n\
         Each Allow applies only to this request. Deny answers are remembered for {origin} \
         until you clear site permissions or close the browser."
    )
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
    use super::{permission_origin, permission_response_allows, prompt_text, SessionDecisions};

    #[test]
    fn the_prompt_says_an_embedded_frame_may_be_asking() {
        let text = prompt_text("https://example.com", "your camera");
        assert!(text
            .starts_with("https://example.com (or a site embedded in it) requests your camera."));
        assert!(text.contains("Each Allow applies only to this request"));
    }

    #[test]
    fn only_denials_are_remembered_per_origin_and_permission() {
        let mut decisions = SessionDecisions::default();
        decisions.remember_denial("https://example.com", "your camera");
        assert!(decisions.is_denied("https://example.com", "your camera"));
        assert!(!decisions.is_denied("https://example.com", "notifications"));
        assert!(!decisions.is_denied("https://other.example", "your camera"));
        decisions.remember_denial("this local file", "your camera");
        assert!(!decisions.is_denied("this local file", "your camera"));
    }

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
