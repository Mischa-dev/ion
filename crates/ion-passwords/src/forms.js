// Ion's login-form helper. Runs in an isolated world (WebEngineScript
// .ApplicationWorld) of each tab's top frame, so the page's own scripts can
// neither see the channel to Ion nor call the functions below. It finds
// password fields, tells Ion when one gets focus (for the list of saved
// logins under it) and when a form with a password is sent (for the save
// prompt), and fills fields when Ion asks through window.__ionForms.
(() => {
    if (window.__ionForms || typeof QWebChannel === "undefined" || !window.qt)
        return;

    let ion = null;
    const queued = [];
    function send(method, ...args) {
        if (ion)
            ion[method](location.href, ...args);
        else
            queued.push([method, args]);
    }
    new QWebChannel(qt.webChannelTransport, channel => {
        ion = channel.objects.ionForms;
        for (const [method, args] of queued.splice(0))
            ion[method](location.href, ...args);
    });

    const TEXTISH = new Set(["text", "email", "tel", "", "username"]);
    const NEW_WORDS = /sign.?up|regist|create|join|new|reset|change|confirm|repeat|again|retype|verify/i;

    function shown(el) {
        if (!el.isConnected || el.disabled || el.readOnly)
            return false;
        const rect = el.getBoundingClientRect();
        if (rect.width < 2 || rect.height < 2)
            return false;
        const style = getComputedStyle(el);
        return style.visibility !== "hidden" && style.display !== "none" && Number(style.opacity) > 0.05;
    }
    const isInput = el => el instanceof HTMLInputElement;
    const isPassword = el => isInput(el) && el.type === "password";
    const isText = el => isInput(el) && TEXTISH.has((el.getAttribute("type") || "").toLowerCase());
    const scopeOf = el => el.form || el.closest("form") || document;
    const passwordsIn = scope => [...scope.querySelectorAll("input[type=password]")].filter(shown);
    const autocomplete = el => (el.getAttribute("autocomplete") || "").toLowerCase();

    // The username field that goes with password field `pw`: one marked as
    // such, else the last text field before it in the same form.
    function usernameFor(pw) {
        const scope = scopeOf(pw);
        const fields = [...scope.querySelectorAll("input")].filter(el => isText(el) && shown(el));
        const marked = fields.find(el => /username|email/.test(autocomplete(el)));
        if (marked)
            return marked;
        let found = null;
        for (const el of fields) {
            if (el.compareDocumentPosition(pw) & Node.DOCUMENT_POSITION_FOLLOWING)
                found = el;
        }
        // A hidden username field the site keeps for password managers
        // (Google's two-step sign in).
        return found || scope.querySelector("input[autocomplete~=username]");
    }

    // Whether password field `pw` wants a new password (sign up, change
    // password) rather than the saved one.
    function wantsNew(pw) {
        const hint = autocomplete(pw);
        if (hint.includes("new-password"))
            return true;
        if (hint.includes("current-password"))
            return false;
        const fields = passwordsIn(scopeOf(pw));
        if (fields.length >= 3)
            return fields.indexOf(pw) > 0;
        if (fields.length === 2)
            return true;
        const form = scopeOf(pw);
        const words = [pw.name, pw.id, form.id || "", form.getAttribute?.("action") || "",
                       form.getAttribute?.("name") || ""].join(" ");
        return NEW_WORDS.test(words);
    }

    // The password field a focused field belongs to: itself, or the first
    // password field after a username field.
    function passwordFor(el) {
        if (isPassword(el))
            return el;
        if (!isText(el))
            return null;
        const fields = passwordsIn(scopeOf(el));
        const pw = fields.find(f => el.compareDocumentPosition(f) & Node.DOCUMENT_POSITION_FOLLOWING);
        return pw && usernameFor(pw) === el ? pw : null;
    }

    let focused = null;   // the field whose list Ion is showing
    let menuOpen = false;
    let filling = false;  // Ion is typing: its focus moves aren't the person's
    function report(el) {
        if (filling)
            return;
        const pw = passwordFor(el);
        if (!pw)
            return;
        focused = el;
        const rect = el.getBoundingClientRect();
        const kind = isPassword(el) ? (wantsNew(el) ? "new" : "password") : "username";
        send("fieldFocused", kind, rect.left, rect.top, rect.width, rect.height);
    }
    function blur() {
        if (focused) {
            focused = null;
            send("fieldBlurred");
        }
    }
    document.addEventListener("focusin", event => report(event.target), true);
    document.addEventListener("mousedown", event => {
        if (event.target === document.activeElement)
            report(event.target);
    }, true);
    document.addEventListener("focusout", () => setTimeout(() => {
        if (focused && document.activeElement !== focused)
            blur();
    }, 150), true);
    addEventListener("scroll", blur, true);
    addEventListener("resize", blur);

    // While Ion's list is open, the arrows, Enter and Escape drive it.
    document.addEventListener("keydown", event => {
        if (menuOpen && focused && event.target === focused
                && ["ArrowDown", "ArrowUp", "Enter", "Escape", "Tab"].includes(event.key)) {
            send("menuKey", event.key);
            if (event.key !== "Tab") {
                event.preventDefault();
                event.stopImmediatePropagation();
            }
            return;
        }
        if (event.key === "Enter" && isInput(event.target) && passwordFor(event.target))
            capture(scopeOf(event.target));
    }, true);

    // Sending a form: the submit event, a click on its button (sites that
    // sign in with fetch() never submit), or leaving the page.
    let lastSent = "";
    let lastUsername = "";
    function capture(scope) {
        const fields = passwordsIn(scope).filter(f => f.value.length > 0);
        if (fields.length === 0)
            return;
        const fresh = fields.filter(wantsNew);
        let password;
        if (fresh.length > 0) {
            // A new password typed twice must match, or nothing is saved yet.
            if (fresh.some(f => f.value !== fresh[0].value))
                return;
            password = fresh[0].value;
        } else {
            password = fields[0].value;
        }
        const userField = usernameFor(fields[0]);
        const username = (userField && userField.value.trim()) || lastUsername;
        const key = username + "\n" + password;
        if (key === lastSent)
            return;
        lastSent = key;
        send("submitted", username, password);
    }
    document.addEventListener("submit", event => capture(scopeOf(event.target)), true);
    document.addEventListener("click", event => {
        const button = event.target.closest?.("button, input[type=submit], input[type=button], [role=button]");
        if (button && passwordsIn(scopeOf(button)).length > 0)
            setTimeout(() => capture(scopeOf(button)), 0);
    }, true);
    addEventListener("pagehide", () => capture(document), true);

    // Two-step sign in (username, then password on the next step): keep the
    // username typed on the first step.
    document.addEventListener("change", event => {
        const el = event.target;
        if (isText(el) && (el.value.includes("@") || /user|login|email/i.test(autocomplete(el) + el.name + el.id)))
            lastUsername = el.value.trim();
    }, true);

    // Tell Ion once the page has a login form, and again if one appears
    // later (single-page apps).
    let announced = false;
    function scan() {
        if (announced || passwordsIn(document).length === 0)
            return;
        announced = true;
        send("formFound");
    }
    let timer = 0;
    new MutationObserver(() => {
        if (!announced && !timer)
            timer = setTimeout(() => { timer = 0; scan(); }, 300);
    }).observe(document.documentElement, { childList: true, subtree: true });
    scan();

    // Set a field's value the way typing does, so frameworks (React, Vue)
    // notice: the isolated world's own setter bypasses any wrapper the page
    // put on the element, then input and change events follow.
    const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
    function type(el, value, focus) {
        if (!el)
            return;
        filling = true;
        try {
            if (focus)
                el.focus({ preventScroll: true });
            setValue.call(el, value);
            el.dispatchEvent(new Event("input", { bubbles: true }));
            el.dispatchEvent(new Event("change", { bubbles: true }));
        } finally {
            filling = false;
        }
    }

    window.__ionForms = {
        // Fill a saved login into the focused form, or the page's first one.
        fill(username, password, onlyEmpty) {
            // As the page loads, only a sign-in form: never a sign-up or
            // change-password one.
            const start = focused && passwordFor(focused) ? passwordFor(focused)
                        : passwordsIn(document).find(f => !wantsNew(f));
            if (!start || (onlyEmpty && wantsNew(start)))
                return false;
            const userField = usernameFor(start);
            if (onlyEmpty && (start.value || (userField && userField.value)))
                return false;
            // Filled as the page loads (onlyEmpty), focus stays put.
            if (userField && username)
                type(userField, username, !onlyEmpty);
            type(start, password, !onlyEmpty);
            if (focused) {
                filling = true;
                focused.focus({ preventScroll: true });
                filling = false;
            }
            return true;
        },
        // Put a generated password into every new-password field of the
        // focused form (the field and its confirmation).
        fillNew(password) {
            const start = focused && passwordFor(focused);
            if (!start)
                return false;
            const back = focused;
            for (const field of passwordsIn(scopeOf(start)).filter(wantsNew))
                type(field, password, true);
            type(back, back.value, true);
            return true;
        },
        menu(open) {
            menuOpen = open;
        },
    };
})();
