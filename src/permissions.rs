use gtk::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use webkit2gtk::{
    GeolocationPermissionRequest, NotificationPermissionRequest, PermissionRequest,
    PermissionRequestExt, PointerLockPermissionRequest, UserMediaPermissionRequest,
    UserMediaPermissionRequestExt, WebView, WebViewExt,
};

thread_local! {
    /// Allow/Deny answers per (origin, permission) for this session only; they
    /// are forgotten when the browser exits (R-8).
    static DECISIONS: RefCell<SessionDecisions> = RefCell::new(SessionDecisions::default());
}

#[derive(Default)]
struct SessionDecisions {
    decisions: HashMap<(String, &'static str), bool>,
}

impl SessionDecisions {
    fn get(&self, origin: &str, permission: &'static str) -> Option<bool> {
        self.decisions.get(&(origin.to_string(), permission)).copied()
    }

    /// Only real origins are remembered; opaque ones ("this page", local
    /// files) are asked about every time.
    fn remember(&mut self, origin: &str, permission: &'static str, allowed: bool) {
        if origin.contains("://") {
            self.decisions.insert((origin.to_string(), permission), allowed);
        }
    }
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
    let remembered = DECISIONS.with(|decisions| decisions.borrow().get(&origin, permission));
    if let Some(allowed) = remembered {
        if allowed {
            request.allow();
        } else {
            request.deny();
        }
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
    // Closing the prompt without choosing is not a decision to remember.
    if matches!(response, gtk::ResponseType::Accept | gtk::ResponseType::Reject) {
        DECISIONS.with(|decisions| decisions.borrow_mut().remember(&origin, permission, allowed));
    }
    if allowed {
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
         Your answer applies to {origin} until you close the browser."
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
        assert!(text.starts_with(
            "https://example.com (or a site embedded in it) requests your camera."
        ));
        assert!(text.contains("until you close the browser"));
    }

    #[test]
    fn decisions_are_remembered_per_origin_and_permission() {
        let mut decisions = SessionDecisions::default();
        decisions.remember("https://example.com", "your camera", true);
        decisions.remember("https://example.com", "notifications", false);
        assert_eq!(decisions.get("https://example.com", "your camera"), Some(true));
        assert_eq!(decisions.get("https://example.com", "notifications"), Some(false));
        assert_eq!(decisions.get("https://example.com", "your location"), None);
        assert_eq!(decisions.get("https://other.example", "your camera"), None);
        decisions.remember("this local file", "your camera", true);
        assert_eq!(decisions.get("this local file", "your camera"), None);
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
