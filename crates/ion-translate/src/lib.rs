//! Page translation, opt-in, through the model Ion Agent is set up with.
//!
//! Ion has no translation engine of its own (Chromium's translate service
//! isn't part of QtWebEngine), so a page is translated only when you ask,
//! by the model you chose for Ion Agent: local (Ollama, llama.cpp) or a
//! service whose key you gave.
//!
//! [`COLLECT_SCRIPT`] gathers the page's visible text nodes, in Ion's world
//! so the page can't see it, and returns them as a JSON array. [`batches`]
//! splits them into requests of a manageable size, [`messages`] builds each
//! request and [`parse_reply`] checks the answer, which [`apply_script`]
//! puts back in place. [`RESTORE_SCRIPT`] brings the original back.

use serde_json::Value;

/// Most text one request carries, in characters.
pub const BATCH_CHARS: usize = 3500;
/// Most text nodes a page sends at all; the rest stays untranslated.
pub const MAX_NODES: usize = 4000;

/// Script (Ion's world, main frame): the page's text, as a JSON array of
/// strings, remembered in order so [`apply_script`] can find each again.
pub const COLLECT_SCRIPT: &str = r#"(() => {
  const skip = new Set(["SCRIPT", "STYLE", "NOSCRIPT", "TEMPLATE", "CODE", "PRE", "KBD", "SAMP",
                        "TEXTAREA", "INPUT", "SELECT", "OPTION", "SVG", "MATH", "CANVAS"]);
  const state = window.__ionTranslate || (window.__ionTranslate = { nodes: [], originals: [] });
  if (state.nodes.length) {
    // Translating again starts from the original text.
    state.nodes.forEach((node, i) => { node.nodeValue = state.originals[i]; });
  }
  state.nodes = []; state.originals = [];
  const walker = document.createTreeWalker(document.body || document.documentElement, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      const text = node.nodeValue;
      if (!text || text.trim().length < 2 || !/\p{L}/u.test(text)) return NodeFilter.FILTER_REJECT;
      for (let el = node.parentElement; el; el = el.parentElement) {
        if (skip.has(el.tagName.toUpperCase()) || el.isContentEditable
            || el.getAttribute("translate") === "no" || el.classList.contains("notranslate"))
          return NodeFilter.FILTER_REJECT;
      }
      return NodeFilter.FILTER_ACCEPT;
    }
  });
  const texts = [];
  while (walker.nextNode() && texts.length < MAX_NODES) {
    state.nodes.push(walker.currentNode);
    state.originals.push(walker.currentNode.nodeValue);
    texts.push(walker.currentNode.nodeValue.trim());
  }
  return JSON.stringify({ lang: document.documentElement.lang || "", texts });
})()"#;

/// Script (Ion's world, main frame) putting the original text back.
pub const RESTORE_SCRIPT: &str = r#"(() => {
  const state = window.__ionTranslate;
  if (!state) return;
  state.nodes.forEach((node, i) => { node.nodeValue = state.originals[i]; });
  state.nodes = []; state.originals = [];
})()"#;

/// The collect script with its limit filled in.
pub fn collect_script() -> String {
    COLLECT_SCRIPT.replace("MAX_NODES", &MAX_NODES.to_string())
}

/// What [`COLLECT_SCRIPT`] found: the page's declared language and its
/// texts. `None` for anything else.
pub fn parse_page(json: &str) -> Option<(String, Vec<String>)> {
    let value: Value = serde_json::from_str(json).ok()?;
    let lang = value["lang"].as_str().unwrap_or("").to_owned();
    let texts = value["texts"]
        .as_array()?
        .iter()
        .map(|t| t.as_str().unwrap_or("").to_owned())
        .collect();
    Some((lang, texts))
}

/// Whether a page declaring `page_lang` is already in `target` (BCP 47
/// tags, compared by their language subtag: `en-GB` is in `en-US`).
pub fn same_language(page_lang: &str, target: &str) -> bool {
    let primary = |tag: &str| {
        tag.split(['-', '_'])
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
    };
    let page = primary(page_lang);
    !page.is_empty() && page == primary(target)
}

/// One request's worth of texts: their indexes in the page and the texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub indexes: Vec<usize>,
    pub texts: Vec<String>,
}

/// Splits `texts` into batches of at most `max_chars` characters each (a
/// single longer text goes alone). Order is kept.
pub fn batches(texts: &[String], max_chars: usize) -> Vec<Batch> {
    let mut out: Vec<Batch> = Vec::new();
    let mut size = 0;
    for (i, text) in texts.iter().enumerate() {
        let len = text.chars().count();
        let full = out
            .last()
            .is_none_or(|b| !b.texts.is_empty() && size + len > max_chars);
        if full {
            out.push(Batch {
                indexes: Vec::new(),
                texts: Vec::new(),
            });
            size = 0;
        }
        let batch = out.last_mut().expect("pushed above");
        batch.indexes.push(i);
        batch.texts.push(text.clone());
        size += len;
    }
    out
}

/// The system and user messages asking a model to translate `batch` into
/// the language tagged `target` (BCP 47, such as `en-US`).
pub fn messages(batch: &Batch, target: &str) -> (String, String) {
    let system = format!(
        "You translate text from a web page into the language with the BCP 47 tag {target}. \
         The user sends a JSON array of strings. Reply with only a JSON array of the same \
         length, each item the translation of the item at the same position. Keep names, \
         numbers, URLs, code and emoji as they are; leave an item unchanged if it is already \
         in that language or can't be translated. No commentary, no code fences."
    );
    let user = Value::from(batch.texts.clone()).to_string();
    (system, user)
}

/// Reads a model's answer for a batch of `expected` texts: a JSON array of
/// that many strings, possibly wrapped in prose or a code fence.
pub fn parse_reply(text: &str, expected: usize) -> Result<Vec<String>, String> {
    let start = text
        .find('[')
        .ok_or("The model didn't answer with a list.")?;
    let end = text
        .rfind(']')
        .ok_or("The model didn't answer with a list.")?;
    if end < start {
        return Err("The model didn't answer with a list.".to_owned());
    }
    let items: Vec<Value> = serde_json::from_str(&text[start..=end])
        .map_err(|_| "The model's answer wasn't a list of texts.".to_owned())?;
    if items.len() != expected {
        return Err(format!(
            "The model answered {} texts for {expected}.",
            items.len()
        ));
    }
    Ok(items
        .into_iter()
        .map(|item| match item {
            Value::String(s) => s,
            other => other.to_string(),
        })
        .collect())
}

/// Script (Ion's world, main frame) putting `translations` in place of the
/// collected texts at `indexes`, keeping each node's surrounding spaces.
pub fn apply_script(indexes: &[usize], translations: &[String]) -> String {
    let pairs: Vec<Value> = indexes
        .iter()
        .zip(translations)
        .map(|(i, t)| Value::from(vec![Value::from(*i), Value::from(t.as_str())]))
        .collect();
    format!(
        r#"(() => {{
  const state = window.__ionTranslate;
  if (!state) return;
  for (const [i, text] of {pairs}) {{
    const node = state.nodes[i];
    if (!node) continue;
    const original = state.originals[i];
    const lead = original.match(/^\s*/)[0], trail = original.match(/\s*$/)[0];
    node.nodeValue = lead + text + trail;
  }}
}})()"#,
        pairs = Value::from(pairs)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn batches_keep_order_and_size() {
        let all = texts(&["aaaa", "bbbb", "cccc", "a very long text indeed"]);
        let out = batches(&all, 9);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].indexes, [0, 1]);
        assert_eq!(out[1].indexes, [2]);
        assert_eq!(out[2].texts, ["a very long text indeed"]);
        assert!(batches(&[], 10).is_empty());
    }

    #[test]
    fn replies_are_checked() {
        assert_eq!(
            parse_reply("```json\n[\"Hola\", \"Mundo\"]\n```", 2).unwrap(),
            ["Hola", "Mundo"]
        );
        assert!(parse_reply("[\"Hola\"]", 2).is_err());
        assert!(parse_reply("Sorry, I can't.", 1).is_err());
        assert_eq!(parse_reply("[1]", 1).unwrap(), ["1"]);
    }

    #[test]
    fn messages_carry_the_texts_as_json() {
        let batch = &batches(&texts(&["Hello \"world\""]), 100)[0];
        let (system, user) = messages(batch, "es-ES");
        assert!(system.contains("es-ES"));
        assert_eq!(user, r#"["Hello \"world\""]"#);
    }

    #[test]
    fn languages_compare_by_primary_subtag() {
        assert!(same_language("en-GB", "en_US"));
        assert!(!same_language("de", "en-US"));
        assert!(!same_language("", "en-US"));
    }

    #[test]
    fn scripts_are_filled_in() {
        assert!(collect_script().contains(&format!("texts.length < {MAX_NODES}")));
        let script = apply_script(&[3], &texts(&["Hola </script>"]));
        assert!(script.contains(r#"[[3,"Hola </script>"]]"#));
        let (lang, found) = parse_page(r#"{"lang":"fr","texts":["Bonjour"]}"#).unwrap();
        assert_eq!((lang.as_str(), found.len()), ("fr", 1));
    }
}
