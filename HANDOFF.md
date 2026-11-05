# Handoff

Branch native-qt6-wayland. Qt 6 / QtWebEngine browser; see README.md for features and layout.

Verified in a nested Hyprland on a headless output with scratch HOME and XDG_DATA_HOME: tabs, omnibox, suggestions, bookmarks, history (record, search, open, clear, persistence), ad blocker, find, zoom, reopen tab, permissions (Allow, Deny not stored, location refused), print to PDF, downloads, fullscreen, popups, themes, assistant, single instance, cookie persistence.

Google sign-in works up to the account password; a real account sign-in needs the owner's credentials. See .trail/learnings for why a Chrome UA plus window.chrome.app is required.
