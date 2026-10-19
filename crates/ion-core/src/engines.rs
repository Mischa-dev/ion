//! The search engines Ion knows by name, with the address each one answers
//! search suggestions from.
//!
//! `search.engine` in config picks one by name. A custom `search.template`
//! still works; its suggestions come from the known engine on the same host,
//! or from `search.suggestTemplate`.

/// One search engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Engine {
    pub name: &'static str,
    /// Results page with `{}` where the query goes.
    pub template: &'static str,
    /// Suggestions endpoint answering in the OpenSearch JSON format
    /// (`["query", ["suggestion", …]]`), with `{}` for the query.
    pub suggest: &'static str,
}

const fn engine(name: &'static str, template: &'static str, suggest: &'static str) -> Engine {
    Engine {
        name,
        template,
        suggest,
    }
}

/// Every engine offered in settings and the palette, the default first.
pub const ENGINES: &[Engine] = &[
    engine(
        "DuckDuckGo",
        "https://duckduckgo.com/?q={}",
        "https://duckduckgo.com/ac/?q={}&type=list",
    ),
    engine(
        "Google",
        "https://www.google.com/search?q={}",
        "https://www.google.com/complete/search?client=firefox&q={}",
    ),
    engine(
        "Bing",
        "https://www.bing.com/search?q={}",
        "https://api.bing.com/osjson.aspx?query={}",
    ),
    engine(
        "Brave Search",
        "https://search.brave.com/search?q={}",
        "https://search.brave.com/api/suggest?q={}",
    ),
    engine(
        "Kagi",
        "https://kagi.com/search?q={}",
        "https://kagi.com/api/autosuggest?q={}",
    ),
    engine(
        "Startpage",
        "https://www.startpage.com/do/search?query={}",
        "https://www.startpage.com/osuggestions?q={}",
    ),
    engine(
        "Ecosia",
        "https://www.ecosia.org/search?q={}",
        "https://ac.ecosia.org/autocomplete?q={}&type=list",
    ),
];

/// The engine called `name`, ignoring case.
pub fn by_name(name: &str) -> Option<&'static Engine> {
    ENGINES
        .iter()
        .find(|engine| engine.name.eq_ignore_ascii_case(name.trim()))
}

/// The engine whose results page lives on the same host as `template`.
pub fn by_template(template: &str) -> Option<&'static Engine> {
    let host = host(template)?;
    ENGINES
        .iter()
        .find(|engine| host_of_static(engine.template) == Some(host.as_str()))
}

/// The suggestions endpoint for the configured engine: `custom` when set,
/// else the known engine matching `template`'s host, else the one called
/// `name`. Empty when there is none.
pub fn suggest_template(name: &str, template: &str, custom: &str) -> String {
    if !custom.trim().is_empty() {
        return custom.trim().to_owned();
    }
    by_template(template)
        .or_else(|| by_name(name).filter(|_| host(template).is_none()))
        .map(|engine| engine.suggest.to_owned())
        .unwrap_or_default()
}

/// The suggestions request for `query`, or `None` without an endpoint.
pub fn suggest_url(suggest_template: &str, query: &str) -> Option<String> {
    let query = query.trim();
    if suggest_template.is_empty() || query.is_empty() || !suggest_template.contains("{}") {
        return None;
    }
    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    Some(suggest_template.replacen("{}", &encoded, 1))
}

fn host(template: &str) -> Option<String> {
    let url = url::Url::parse(&template.replace("{}", "q")).ok()?;
    url.host_str()
        .map(|h| h.trim_start_matches("www.").to_owned())
}

fn host_of_static(template: &str) -> Option<&str> {
    let rest = template.split_once("://")?.1;
    let host = rest.split(['/', '?']).next()?;
    Some(host.trim_start_matches("www."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_engines_by_name_and_host() {
        assert_eq!(by_name("google").unwrap().name, "Google");
        assert_eq!(by_name("Nope"), None);
        assert_eq!(
            by_template("https://www.google.com/search?q={}&hl=en")
                .unwrap()
                .name,
            "Google"
        );
        assert_eq!(
            by_template("https://google.com/search?q={}").unwrap().name,
            "Google"
        );
        assert_eq!(by_template("https://example.com/?q={}"), None);
    }

    #[test]
    fn picks_the_suggestion_endpoint() {
        assert_eq!(
            suggest_template("DuckDuckGo", "https://duckduckgo.com/?q={}", ""),
            "https://duckduckgo.com/ac/?q={}&type=list"
        );
        // The template decides when it names a known host.
        assert_eq!(
            suggest_template("DuckDuckGo", "https://www.google.com/search?q={}", ""),
            ENGINES[1].suggest
        );
        // A custom engine has no suggestions unless configured.
        assert_eq!(
            suggest_template("Mine", "https://search.example/?q={}", ""),
            ""
        );
        assert_eq!(
            suggest_template(
                "Mine",
                "https://search.example/?q={}",
                " https://search.example/s?q={} "
            ),
            "https://search.example/s?q={}"
        );
    }

    #[test]
    fn builds_suggestion_requests() {
        assert_eq!(
            suggest_url(ENGINES[0].suggest, " rust & qt ").as_deref(),
            Some("https://duckduckgo.com/ac/?q=rust+%26+qt&type=list")
        );
        assert_eq!(suggest_url(ENGINES[0].suggest, "  "), None);
        assert_eq!(suggest_url("", "rust"), None);
    }
}
