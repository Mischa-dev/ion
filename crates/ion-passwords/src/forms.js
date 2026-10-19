// Ion's login-form helper. Runs in an isolated world (WebEngineScript
// .ApplicationWorld) of each tab's top frame, so the page's own scripts can
// neither see the channel to Ion nor call the functions below. It finds
// password fields, tells Ion when one gets focus (for the list of saved
// logins under it) and when a form with a password is sent (for the save
// prompt), and fills fields when Ion asks through window.__ionForms. It
// does the same for address forms (shipping, billing, contact details).
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

    // Address forms. Each field gets a part: the `autocomplete` token when
    // the site gives one, else a guess from its name, id, placeholder and
    // label.
    const ADDRESS_TOKENS = {
        "name": "name", "given-name": "given", "additional-name": "middle", "family-name": "family",
        "organization": "organization", "street-address": "street", "address-line1": "line1",
        "address-line2": "line2", "address-line3": "line3", "address-level2": "city",
        "address-level1": "region", "postal-code": "postal", "country": "country",
        "country-name": "country", "email": "email", "tel": "phone", "tel-national": "phone",
    };
    const NOT_ADDRESS = /^(cc-|one-time-code|username|current-password|new-password|off$|nope)/;
    const GUESSES = [
        [/e.?mail/, "email"],
        [/phone|mobile|\btel\b|telephone/, "phone"],
        [/full.?name/, "name"],
        [/first.?name|given.?name|\bfname\b|forename/, "given"],
        [/last.?name|family.?name|surname|\blname\b/, "family"],
        [/company|organi[sz]ation|business.?name/, "organization"],
        [/address.?(line)?.?[23]|apartment|suite|\bapt\b|flat.?number/, "line2"],
        [/street|address.?(line)?.?1|\baddress\b|addr/, "line1"],
        [/city|town|locality|suburb/, "city"],
        [/state|province|region|county/, "region"],
        [/zip|postal|post.?code/, "postal"],
        [/country/, "country"],
        [/\bname\b|recipient/, "name"],
    ];
    const SKIP_TYPES = /^(hidden|password|search|checkbox|radio|submit|button|reset|image|file|range|color|date|datetime-local|month|week|time|number)$/;
    function addressPart(el) {
        if (!(isInput(el) || el instanceof HTMLSelectElement || el instanceof HTMLTextAreaElement))
            return null;
        if (isInput(el) && SKIP_TYPES.test((el.getAttribute("type") || "").toLowerCase()))
            return null;
        // "shipping street-address", "section-a billing postal-code"
        const tokens = autocomplete(el).split(/\s+/).filter(t => t && !/^(section-|shipping|billing|home|work|mobile)/.test(t));
        const token = tokens[tokens.length - 1] || "";
        if (ADDRESS_TOKENS[token])
            return ADDRESS_TOKENS[token];
        if (token && NOT_ADDRESS.test(token) && token !== "off")
            return null;
        const label = [...(el.labels || [])].map(l => l.textContent).join(" ");
        const words = [el.name, el.id, el.getAttribute("placeholder") || "",
                       el.getAttribute("aria-label") || "", label].join(" ").toLowerCase();
        if (/user.?name|login|search|coupon|promo|card|captcha|password/.test(words))
            return null;
        if (isInput(el) && el.type === "email")
            return "email";
        if (isInput(el) && el.type === "tel")
            return "phone";
        for (const [pattern, part] of GUESSES) {
            if (pattern.test(words))
                return part;
        }
        return null;
    }
    // The address fields of `scope` as [element, part] pairs, or [] unless
    // it looks like an address form: three different parts, and no
    // password field (logins and sign-ups are the password side's).
    function addressFields(scope) {
        if (passwordsIn(scope).length > 0)
            return [];
        const fields = [...scope.querySelectorAll("input, select, textarea")]
            .filter(shown)
            .map(el => [el, addressPart(el)])
            .filter(([, part]) => part);
        return new Set(fields.map(([, part]) => part)).size >= 3 ? fields : [];
    }
    const inAddressForm = el => addressFields(scopeOf(el)).some(([field]) => field === el);

    let focused = null;   // the field whose list Ion is showing
    let menuOpen = false;
    let filling = false;  // Ion is typing: its focus moves aren't the person's
    function report(el) {
        if (filling)
            return;
        const pw = passwordFor(el);
        const address = !pw && inAddressForm(el);
        if (!pw && !address)
            return;
        focused = el;
        const rect = el.getBoundingClientRect();
        const kind = address ? "address" : isPassword(el) ? (wantsNew(el) ? "new" : "password") : "username";
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
        if (event.key === "Enter" && isInput(event.target)) {
            if (passwordFor(event.target))
                capture(scopeOf(event.target));
            else if (inAddressForm(event.target))
                captureAddress(scopeOf(event.target));
        }
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

    // An address form sent: its details, for "Save this address?".
    let lastAddress = "";
    function captureAddress(scope) {
        const fields = addressFields(scope);
        if (fields.length === 0)
            return;
        const got = {};
        for (const [el, part] of fields) {
            const value = (el instanceof HTMLSelectElement
                ? (el.selectedOptions[0]?.textContent || "") : el.value).trim();
            if (value && !got[part])
                got[part] = value;
        }
        const address = {
            name: got.name || [got.given, got.middle, got.family].filter(Boolean).join(" "),
            organization: got.organization || "",
            street: got.street || [got.line1, got.line2, got.line3].filter(Boolean).join("\n"),
            city: got.city || "",
            region: got.region || "",
            postalCode: got.postal || "",
            country: got.country || "",
            email: got.email || "",
            phone: got.phone || "",
        };
        const json = JSON.stringify(address);
        if (json === lastAddress)
            return;
        lastAddress = json;
        send("addressSubmitted", json);
    }

    function captureAny(scope) {
        capture(scope);
        captureAddress(scope);
    }
    document.addEventListener("submit", event => captureAny(scopeOf(event.target)), true);
    document.addEventListener("click", event => {
        const button = event.target.closest?.("button, input[type=submit], input[type=button], [role=button]");
        if (button)
            setTimeout(() => captureAny(scopeOf(button)), 0);
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
    const setters = [HTMLInputElement, HTMLTextAreaElement, HTMLSelectElement]
        .map(kind => [kind, Object.getOwnPropertyDescriptor(kind.prototype, "value").set]);
    function type(el, value, focus) {
        if (!el)
            return;
        filling = true;
        try {
            if (focus)
                el.focus({ preventScroll: true });
            setters.find(([kind]) => el instanceof kind)[1].call(el, value);
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
        // Fill a saved address into the focused address form: every empty
        // field, and the focused one.
        fillAddress(address) {
            const fields = focused ? addressFields(scopeOf(focused)) : [];
            if (fields.length === 0)
                return false;
            const names = address.name.trim().split(/\s+/);
            const lines = address.street.split("\n").map(l => l.trim()).filter(Boolean);
            const values = {
                name: address.name,
                given: names.length > 1 ? names.slice(0, -1).join(" ") : names[0],
                family: names.length > 1 ? names[names.length - 1] : "",
                middle: "",
                organization: address.organization,
                street: lines.join("\n"),
                line1: lines[0] || "",
                line2: lines.slice(1).join(", "),
                line3: "",
                city: address.city,
                region: address.region,
                postal: address.postalCode,
                country: address.country,
                email: address.email,
                phone: address.phone,
            };
            const back = focused;
            for (const [el, part] of fields) {
                let value = values[part] || "";
                if (!value || (el.value && el !== back))
                    continue;
                if (el instanceof HTMLSelectElement) {
                    // Options are often codes ("CA") with names ("California").
                    const want = value.toLowerCase();
                    const option = [...el.options].find(o => o.value.toLowerCase() === want
                                                           || o.textContent.trim().toLowerCase() === want)
                        || [...el.options].find(o => o.value && o.textContent.trim().toLowerCase().startsWith(want));
                    if (!option)
                        continue;
                    value = option.value;
                } else if (el instanceof HTMLTextAreaElement && part === "line1") {
                    value = values.street;
                } else if (isInput(el) && part === "street") {
                    value = lines.join(", ");
                }
                type(el, value, false);
            }
            filling = true;
            back.focus({ preventScroll: true });
            filling = false;
            return true;
        },
        menu(open) {
            menuOpen = open;
        },
    };
})();
