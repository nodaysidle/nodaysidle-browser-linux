# AGENTS.md — nodaysidle-browser-linux

- Never add a library dependency (`QT +=` or `LIBS` in `nodaysidle-browser.pro`, `depends` in `pkgbuild/PKGBUILD`) without explicit user permission.
- Never pass `--no-sandbox` to QtWebEngine; the renderer sandbox stays on. Zero telemetry: no tracking, analytics beacons or remote logging.
