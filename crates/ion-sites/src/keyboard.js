// Ion keyboard mode: Vim-style keys and link hints, injected into every page
// when `[keyboard] vim = true`. Runs in an isolated world; keys typed into
// text fields and editable areas are left alone.
//
//   j / k        scroll down / up          d / u   half a page down / up
//   gg / G       top / bottom              H / L   back / forward
//   f            follow a link             F       open a link in a new tab
//   Escape       cancel hints
(() => {
  if (window.__ionKeyboard) return;
  window.__ionKeyboard = true;

  const STEP = 60;
  const ALPHABET = "asdfghjkl";
  let pending = "";
  let hints = null;

  const editable = (el) => {
    if (!el) return false;
    if (el.isContentEditable) return true;
    const tag = el.tagName;
    if (tag === "TEXTAREA" || tag === "SELECT") return true;
    if (tag !== "INPUT") return false;
    const type = (el.type || "text").toLowerCase();
    return !["button", "checkbox", "radio", "submit", "reset", "image", "range", "color", "file"].includes(type);
  };

  // Prefix-free labels: the shortest length that fits `n` links.
  const labels = (n) => {
    let length = 1;
    while (Math.pow(ALPHABET.length, length) < n) length++;
    const out = [];
    for (let i = 0; i < n; i++) {
      let label = "";
      let k = i;
      for (let j = 0; j < length; j++) {
        label = ALPHABET[k % ALPHABET.length] + label;
        k = Math.floor(k / ALPHABET.length);
      }
      out.push(label);
    }
    return out;
  };

  const visible = (el) => {
    const r = el.getBoundingClientRect();
    if (r.width < 2 || r.height < 2) return null;
    if (r.bottom < 0 || r.right < 0 || r.top > innerHeight || r.left > innerWidth) return null;
    const style = getComputedStyle(el);
    if (style.visibility === "hidden" || style.display === "none" || style.opacity === "0") return null;
    return r;
  };

  const clearHints = () => {
    if (hints) hints.layer.remove();
    hints = null;
  };

  const showHints = (newTab) => {
    clearHints();
    const targets = [];
    for (const el of document.querySelectorAll(
      "a[href], button, input, select, textarea, summary, [role=button], [role=link], [onclick], [tabindex]:not([tabindex='-1'])"
    )) {
      const rect = visible(el);
      if (rect) targets.push({ el, rect });
    }
    if (targets.length === 0) return;
    const layer = document.createElement("div");
    layer.setAttribute("data-ion", "hints");
    layer.style.cssText = "position:fixed;inset:0;z-index:2147483647;pointer-events:none;";
    const names = labels(targets.length);
    targets.forEach((t, i) => {
      const tag = document.createElement("span");
      tag.textContent = names[i].toUpperCase();
      tag.style.cssText =
        "position:fixed;padding:1px 4px;border-radius:4px;font:bold 11px/1.3 ui-monospace,monospace;" +
        "background:#1e1e2e;color:#f9e2af;border:1px solid #f9e2af;box-shadow:0 1px 3px rgba(0,0,0,.4);" +
        `left:${Math.max(0, t.rect.left)}px;top:${Math.max(0, t.rect.top)}px;`;
      layer.appendChild(tag);
      t.label = names[i];
      t.tag = tag;
    });
    (document.body || document.documentElement).appendChild(layer);
    hints = { layer, targets, typed: "", newTab };
  };

  const activate = (el, newTab) => {
    const href = el.tagName === "A" ? el.href : null;
    if (newTab && href) {
      window.open(href, "_blank", "noopener");
      return;
    }
    if (editable(el) || el.tagName === "SELECT") {
      el.focus();
      return;
    }
    el.focus();
    el.click();
  };

  const hintKey = (key) => {
    if (key === "Escape") {
      clearHints();
      return;
    }
    if (key === "Backspace") {
      hints.typed = hints.typed.slice(0, -1);
    } else if (key.length === 1 && ALPHABET.includes(key.toLowerCase())) {
      hints.typed += key.toLowerCase();
    } else {
      return;
    }
    const matches = hints.targets.filter((t) => t.label.startsWith(hints.typed));
    for (const t of hints.targets) t.tag.style.display = t.label.startsWith(hints.typed) ? "" : "none";
    if (matches.length === 1 && matches[0].label === hints.typed) {
      const { newTab } = hints;
      clearHints();
      activate(matches[0].el, newTab);
    } else if (matches.length === 0) {
      clearHints();
    }
  };

  const scroller = () => document.scrollingElement || document.documentElement;

  addEventListener(
    "keydown",
    (event) => {
      if (event.ctrlKey || event.metaKey || event.altKey) return;
      if (hints) {
        event.preventDefault();
        event.stopImmediatePropagation();
        hintKey(event.key);
        return;
      }
      if (editable(event.target) || editable(document.activeElement)) return;
      const key = pending + event.key;
      pending = "";
      let handled = true;
      switch (key) {
        case "j": scrollBy({ top: STEP }); break;
        case "k": scrollBy({ top: -STEP }); break;
        case "d": scrollBy({ top: innerHeight / 2 }); break;
        case "u": scrollBy({ top: -innerHeight / 2 }); break;
        case "gg": scroller().scrollTo({ top: 0 }); break;
        case "G": scroller().scrollTo({ top: scroller().scrollHeight }); break;
        case "H": history.back(); break;
        case "L": history.forward(); break;
        case "f": showHints(false); break;
        case "F": showHints(true); break;
        case "g": pending = "g"; break;
        default: handled = false;
      }
      if (handled) {
        event.preventDefault();
        event.stopImmediatePropagation();
      }
    },
    true
  );
  addEventListener("scroll", clearHints, true);
})();
