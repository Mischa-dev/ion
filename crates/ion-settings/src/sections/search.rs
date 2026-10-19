use ion_config::Value;

use crate::{Context, Control, Group, Preset, Section, Setting};

/// Engines offered with one click; anything else goes in the URL field.
pub const ENGINES: &[(&str, &str)] = &[
    ("DuckDuckGo", "https://duckduckgo.com/?q={}"),
    ("Google", "https://www.google.com/search?q={}"),
    ("Brave Search", "https://search.brave.com/search?q={}"),
    ("Kagi", "https://kagi.com/search?q={}"),
    ("Startpage", "https://www.startpage.com/do/search?query={}"),
    ("Ecosia", "https://www.ecosia.org/search?q={}"),
    ("Bing", "https://www.bing.com/search?q={}"),
];

pub fn section(_: &Context) -> Section {
    let presets = ENGINES
        .iter()
        .map(|&(name, template)| Preset {
            label: name.into(),
            values: vec![
                ("search.engine".into(), Value::String(name.into())),
                ("search.template".into(), Value::String(template.into())),
            ],
        })
        .collect();
    Section::new(
        "search",
        "Search",
        "⌕",
        "What the address bar does with words that aren't an address.",
        vec![Group::new(
            "Search engine",
            vec![
                Setting::new(
                    "search.engine",
                    "Search with",
                    Control::Preset { options: presets },
                )
                .keywords("default engine google duckduckgo"),
                Setting::new(
                    "search.template",
                    "Search address",
                    Control::Text {
                        placeholder: "https://example.com/search?q={}".into(),
                    },
                )
                .help("For any other engine: its search URL, with {} where your words go.")
                .keywords("custom url template query"),
            ],
        )],
    )
}
