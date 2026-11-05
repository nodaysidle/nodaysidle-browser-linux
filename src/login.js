// Runs in an isolated world on every top-level page, so page scripts cannot see or forge it.
// It finds login forms, reports a submitted login and fills a saved one. Messages reach the
// browser as console lines that start with the per-run nonce (__ndNonce, set by the host).
(function () {
    const send = (message) => console.log(__ndNonce + JSON.stringify(message));
    const USER_KEY = '__nd_user';
    const textTypes = new Set(['text', 'email', 'tel', '']);

    function visible(el) {
        if (!el || el.disabled || el.readOnly)
            return false;
        const box = el.getBoundingClientRect();
        if (box.width < 4 || box.height < 4)
            return false;
        const style = getComputedStyle(el);
        return style.visibility !== 'hidden' && style.display !== 'none';
    }

    const passwordFields = () => Array.from(document.querySelectorAll('input[type=password]')).filter(visible);

    function looksLikeUser(field) {
        return /\b(username|email)\b/.test(field.autocomplete) || field.type === 'email' ||
            /user|email|login|ident|account/i.test(field.name + ' ' + field.id);
    }

    // The visible text field just before the password field, or on identifier-first pages
    // the field marked autocomplete=username.
    function usernameField(pw) {
        const scope = (pw && pw.form) || document;
        const fields = Array.from(scope.querySelectorAll('input'))
            .filter((f) => textTypes.has((f.getAttribute('type') || '').toLowerCase()) && visible(f));
        if (!pw)
            return fields.find((f) => /\busername\b/.test(f.autocomplete)) || null;
        let found = null;
        for (const f of fields)
            if (f.compareDocumentPosition(pw) & Node.DOCUMENT_POSITION_FOLLOWING)
                found = f;
        return found;
    }

    // Multi-step sign-ins ask for the account first and the password on the next screen.
    function rememberedUser() {
        try { return sessionStorage.getItem(USER_KEY) || ''; } catch (e) { return ''; }
    }
    document.addEventListener('input', (e) => {
        const field = e.target;
        if (field instanceof HTMLInputElement && textTypes.has((field.getAttribute('type') || '').toLowerCase()) &&
            field.value && looksLikeUser(field)) {
            try { sessionStorage.setItem(USER_KEY, field.value); } catch (err) { /* storage can be blocked */ }
        }
    }, true);

    function setValue(field, value) {
        // The native setter keeps frameworks that track values (React) from ignoring the change.
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(field, value);
        field.dispatchEvent(new Event('input', { bubbles: true }));
        field.dispatchEvent(new Event('change', { bubbles: true }));
    }

    window.__ndFill = function (user, pass) {
        const pw = pass === null ? null : passwordFields().find((f) => f.autocomplete !== 'new-password' && !f.value);
        const userField = usernameField(pw);
        if (user && userField && !userField.value)
            setValue(userField, user);
        if (pw)
            setValue(pw, pass);
    };

    const asked = new WeakSet();
    let lastHasPassword = null;
    let scanTimer = 0;

    function scan() {
        const all = passwordFields();
        const empty = all.find((f) => f.autocomplete !== 'new-password' && !f.value);
        if (empty && !asked.has(empty)) {
            asked.add(empty);
            const userField = usernameField(empty);
            send({ t: 'need', pw: true, user: (userField && userField.value) || rememberedUser() });
        } else if (all.length === 0) {
            const userField = usernameField(null);
            if (userField && !userField.value && !asked.has(userField)) {
                asked.add(userField);
                send({ t: 'need', pw: false, user: '' });
            }
        }
        if (lastHasPassword !== all.length > 0) {
            lastHasPassword = all.length > 0;
            send({ t: 'forms', pw: lastHasPassword });
        }
    }
    const scheduleScan = () => { clearTimeout(scanTimer); scanTimer = setTimeout(scan, 250); };

    let lastKey = '';
    let lastAt = 0;

    function report(pw) {
        if (!pw.value)
            return;
        const userField = usernameField(pw);
        const user = (userField && userField.value) || rememberedUser();
        const key = user + '\n' + pw.value;
        if (key === lastKey && Date.now() - lastAt < 1500)
            return;
        lastKey = key;
        lastAt = Date.now();
        send({ t: 'save', user: user, pass: pw.value });
        // A form that stays on screen means the sign-in did not go through.
        let ticks = 0;
        const timer = setInterval(() => {
            if (!pw.isConnected || !visible(pw)) {
                clearInterval(timer);
                send({ t: 'gone' });
            } else if (++ticks > 25) {
                clearInterval(timer);
                send({ t: 'stuck' });
            }
        }, 400);
    }

    document.addEventListener('submit', (e) => {
        const pw = e.target.querySelector && e.target.querySelector('input[type=password]');
        if (pw)
            report(pw);
    }, true);
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && e.target instanceof HTMLInputElement && e.target.type === 'password')
            report(e.target);
    }, true);
    document.addEventListener('click', (e) => {
        const button = e.target instanceof Element ? e.target.closest('button, input[type=submit], input[type=button], [role=button], a') : null;
        if (!button)
            return;
        const label = [button.id, button.getAttribute('class'), button.getAttribute('aria-label'), button.title, button.textContent.slice(0, 40)].join(' ');
        if (/show|hide|reveal|toggle|visib|eye/i.test(label))
            return;
        const pw = passwordFields().find((f) => f.value);
        if (pw)
            report(pw);
    }, true);

    new MutationObserver(scheduleScan).observe(document, { childList: true, subtree: true });
    scheduleScan();
})();
