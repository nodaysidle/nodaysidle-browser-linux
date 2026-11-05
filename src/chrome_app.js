// QtWebEngine never defines window.chrome.app, which Chrome gives every page.
// Google's sign-in reads it and answers "This browser or app may not be secure"
// (rrk=46) when it is missing. This is the object Chrome returns for a page
// that is not an installed app.
(function () {
    if (!window.chrome) window.chrome = {};
    if (window.chrome.app) return;
    window.chrome.app = {
        isInstalled: false,
        InstallState: { DISABLED: 'disabled', INSTALLED: 'installed', NOT_INSTALLED: 'not_installed' },
        RunningState: { CANNOT_RUN: 'cannot_run', READY_TO_RUN: 'ready_to_run', RUNNING: 'running' },
        getDetails: function getDetails() { return null; },
        getIsInstalled: function getIsInstalled() { return false; },
        installState: function installState(callback) { if (callback) callback('not_installed'); },
        runningState: function runningState() { return 'cannot_run'; }
    };
})();
