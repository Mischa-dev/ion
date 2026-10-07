//! Finding the readable article in a page.

use std::collections::HashMap;

use ego_tree::{NodeId, NodeRef};
use scraper::node::Node;
use scraper::{ElementRef, Html, Selector};
use url::Url;

/// Pages with less article text than this have nothing worth a reader view.
const MIN_TEXT: usize = 250;
/// Paragraphs shorter than this don't count towards a container's score.
const MIN_PARAGRAPH: usize = 25;

/// The readable part of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Article {
    pub title: String,
    /// Author, from `<meta name="author">`, possibly empty.
    pub byline: String,
    /// Site name, from `og:site_name` or the page's host.
    pub site: String,
    /// Sanitized HTML of the article body.
    pub content: String,
    /// Characters of text in the article, for a reading-time estimate.
    pub text_len: usize,
}

impl Article {
    /// Minutes to read at about 230 words (1200 characters) a minute.
    pub fn minutes(&self) -> usize {
        self.text_len.div_ceil(1200).max(1)
    }
}

/// Elements dropped with everything inside them.
const DROP: &[&str] = &[
    "script", "style", "noscript", "iframe", "form", "button", "input", "select", "textarea",
    "nav", "aside", "footer", "header", "svg", "canvas", "object", "embed", "template", "dialog",
    "menu", "link", "meta", "head", "title",
];

/// Elements kept as they are (attributes filtered).
const KEEP: &[&str] = &[
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "ul",
    "ol",
    "li",
    "blockquote",
    "pre",
    "code",
    "em",
    "strong",
    "b",
    "i",
    "u",
    "s",
    "sub",
    "sup",
    "small",
    "mark",
    "a",
    "img",
    "figure",
    "figcaption",
    "table",
    "thead",
    "tbody",
    "tfoot",
    "tr",
    "td",
    "th",
    "caption",
    "br",
    "hr",
    "dl",
    "dt",
    "dd",
    "q",
    "cite",
    "abbr",
    "time",
    "kbd",
    "samp",
    "var",
    "del",
    "ins",
    "picture",
];

/// Words in a class or id that mark page furniture, not article.
const UNLIKELY: &[&str] = &[
    "comment",
    "sidebar",
    "footer",
    "masthead",
    "menu",
    "nav",
    "share",
    "social",
    "related",
    "promo",
    "sponsor",
    "advert",
    "banner",
    "cookie",
    "newsletter",
    "subscribe",
    "popup",
    "modal",
    "breadcrumb",
    "pagination",
    "skip-link",
];

const VOID: &[&str] = &["img", "br", "hr"];

/// Extract the article from `html`, loaded from `page_url`. `None` when the
/// page has too little prose to be worth reading this way.
pub fn extract(html: &str, page_url: &str) -> Option<Article> {
    let doc = Html::parse_document(html);
    let base = Url::parse(page_url).ok();

    let best = best_container(&doc)?;
    let page_title = first_text(&doc, "title").unwrap_or_default();
    // The article's own heading usually repeats the page title (minus the
    // site name); show it once, as the reader page's title.
    let heading = Selector::parse("h1").ok().and_then(|s| {
        best.select(&s).next().filter(|h| {
            let text = collapse(&h.text().collect::<String>());
            !text.is_empty() && page_title.contains(&text)
        })
    });
    let mut content = String::new();
    let mut text_len = 0;
    for child in best.children() {
        serialize(
            child,
            heading.map(|h| h.id()),
            base.as_ref(),
            &mut content,
            &mut text_len,
        );
    }
    if text_len < MIN_TEXT {
        return None;
    }

    let meta = |selector: &str| {
        Selector::parse(selector).ok().and_then(|s| {
            doc.select(&s)
                .filter_map(|e| e.value().attr("content"))
                .map(collapse)
                .find(|t| !t.is_empty())
        })
    };
    let title = heading
        .map(|h| collapse(&h.text().collect::<String>()))
        .or_else(|| meta(r#"meta[property="og:title"]"#))
        .or_else(|| Some(page_title).filter(|t| !t.is_empty()))
        .or_else(|| first_text(&doc, "h1"))
        .unwrap_or_default();
    let site = meta(r#"meta[property="og:site_name"]"#)
        .or_else(|| {
            base.as_ref()
                .and_then(|u| u.host_str())
                .map(|h| h.trim_start_matches("www.").to_owned())
        })
        .unwrap_or_default();
    Some(Article {
        title,
        byline: meta(r#"meta[name="author"]"#).unwrap_or_default(),
        site,
        content,
        text_len,
    })
}

/// The text of the first non-empty element matching `selector`.
fn first_text(doc: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    doc.select(&selector)
        .map(|e| collapse(&e.text().collect::<String>()))
        .find(|t| !t.is_empty())
}

/// The element holding most of the page's prose.
fn best_container(doc: &Html) -> Option<ElementRef<'_>> {
    let paragraphs = Selector::parse("p, pre, blockquote").ok()?;
    let mut scores: HashMap<NodeId, f64> = HashMap::new();
    for p in doc.select(&paragraphs) {
        if p.ancestors().filter_map(ElementRef::wrap).any(is_furniture) {
            continue;
        }
        let text = collapse(&p.text().collect::<String>());
        let len = text.chars().count();
        if len < MIN_PARAGRAPH {
            continue;
        }
        let score = 1.0 + text.matches(',').count() as f64 + (len as f64 / 100.0).min(3.0);
        let mut ancestors = p.ancestors().filter(|n| n.value().is_element());
        if let Some(parent) = ancestors.next() {
            *scores.entry(parent.id()).or_default() += score;
        }
        if let Some(grandparent) = ancestors.next() {
            *scores.entry(grandparent.id()).or_default() += score / 2.0;
        }
    }
    scores
        .into_iter()
        .filter_map(|(id, score)| {
            let el = ElementRef::wrap(doc.tree.get(id)?)?;
            let boost = match el.value().name() {
                "article" | "main" => 1.25,
                _ => 1.0,
            };
            Some((score * boost * (1.0 - link_density(el)), el))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, el)| el)
}

/// Whether a class or id marks `el` as navigation, comments, ads and the like.
fn is_furniture(el: ElementRef) -> bool {
    let v = el.value();
    if DROP.contains(&v.name()) {
        return true;
    }
    let names = format!(
        "{} {}",
        v.attr("class").unwrap_or(""),
        v.attr("id").unwrap_or("")
    )
    .to_ascii_lowercase();
    // "article-body" mentioning "ad" etc. must not count, so match whole
    // words or word prefixes only.
    names
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .any(|word| UNLIKELY.iter().any(|u| word.starts_with(u)))
        || v.attr("role")
            .is_some_and(|r| matches!(r, "navigation" | "complementary" | "banner" | "contentinfo"))
        || v.attr("aria-hidden") == Some("true")
        || v.attr("hidden").is_some()
}

/// Share of an element's text that sits inside links.
fn link_density(el: ElementRef) -> f64 {
    let total: usize = el.text().map(str::len).sum();
    if total == 0 {
        return 0.0;
    }
    let links = Selector::parse("a").expect("valid selector");
    let linked: usize = el.select(&links).flat_map(|a| a.text()).map(str::len).sum();
    (linked as f64 / total as f64).min(1.0)
}

/// Write `node` as sanitized HTML, leaving out `skip` (the heading used as
/// the title).
fn serialize(
    node: NodeRef<'_, Node>,
    skip: Option<NodeId>,
    base: Option<&Url>,
    out: &mut String,
    text_len: &mut usize,
) {
    if Some(node.id()) == skip {
        return;
    }
    match node.value() {
        Node::Text(text) => {
            *text_len += text.trim().chars().count();
            escape_into(&text.text, out);
        }
        Node::Element(_) => {
            let Some(el) = ElementRef::wrap(node) else {
                return;
            };
            if is_furniture(el) {
                return;
            }
            let name = el.value().name();
            // Link lists (tag clouds, "more stories") inside the article.
            if matches!(name, "ul" | "ol" | "div" | "section" | "table")
                && link_density(el) > 0.5
                && el.text().map(str::len).sum::<usize>() < 500
            {
                return;
            }
            if !KEEP.contains(&name) {
                for child in node.children() {
                    serialize(child, skip, base, out, text_len);
                }
                // Keep block boundaries between flattened containers.
                if matches!(name, "div" | "section" | "article" | "main") {
                    out.push('\n');
                }
                return;
            }
            let attrs = attributes(el, base);
            if name == "img" && attrs.is_empty() {
                return;
            }
            out.push('<');
            out.push_str(name);
            out.push_str(&attrs);
            out.push('>');
            if VOID.contains(&name) {
                return;
            }
            for child in node.children() {
                serialize(child, skip, base, out, text_len);
            }
            out.push_str("</");
            out.push_str(name);
            out.push('>');
        }
        _ => {}
    }
}

/// The attributes kept on `el`, already escaped, with URLs made absolute.
/// Empty for an image without a usable source.
fn attributes(el: ElementRef, base: Option<&Url>) -> String {
    let v = el.value();
    let mut out = String::new();
    let mut push = |key: &str, value: &str| {
        out.push(' ');
        out.push_str(key);
        out.push_str("=\"");
        escape_into(value, &mut out);
        out.push('"');
    };
    match v.name() {
        "a" => {
            if let Some(href) = v.attr("href").and_then(|h| absolute(h, base)) {
                push("href", &href);
            }
        }
        "img" => {
            // Lazy-loading pages keep the real source in data-src.
            let src = ["data-src", "data-original", "src"]
                .iter()
                .filter_map(|k| v.attr(k))
                .find(|s| !s.starts_with("data:"))
                .and_then(|s| absolute(s, base));
            let Some(src) = src else {
                return String::new();
            };
            push("src", &src);
            if let Some(alt) = v.attr("alt") {
                push("alt", alt);
            }
        }
        "td" | "th" => {
            for key in ["colspan", "rowspan"] {
                if let Some(value) = v.attr(key).filter(|n| n.parse::<u16>().is_ok()) {
                    push(key, value);
                }
            }
        }
        _ => {}
    }
    out
}

/// `link` resolved against `base`, if it's a web or mail link.
fn absolute(link: &str, base: Option<&Url>) -> Option<String> {
    let url = match base {
        Some(base) => base.join(link.trim()).ok()?,
        None => Url::parse(link.trim()).ok()?,
    };
    matches!(url.scheme(), "http" | "https" | "mailto" | "file").then(|| url.to_string())
}

/// Collapse runs of whitespace to single spaces and trim.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn escape_into(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(body: &str) -> String {
        format!(
            r#"<html><head><title>Fallback title</title>
            <meta property="og:title" content="The  Real Title">
            <meta name="author" content="Ada Lovelace">
            </head><body>{body}</body></html>"#
        )
    }

    const PROSE: &str = "Long paragraphs of prose, with commas, are what readers want to read, \
        and this one goes on for a while so that it clearly counts as article text.";

    fn article_body() -> String {
        format!(
            r#"<nav class="site-nav"><a href="/">Home</a><a href="/about">About</a></nav>
            <div class="sidebar"><p>{PROSE}</p></div>
            <article>
              <h1>Heading</h1>
              <p>{PROSE} <a href="/more" onclick="steal()">more</a></p>
              <script>alert(1)</script>
              <p style="color:red">{PROSE}</p>
              <img data-src="img/a.png" src="data:image/gif;base64,R0lG" alt="A &quot;pic&quot;">
              <p>{PROSE}</p>
              <ul class="tags"><li><a href="/t/1">one</a></li><li><a href="/t/2">two</a></li></ul>
              <div class="comments"><p>{PROSE}</p></div>
            </article>
            <footer><p>{PROSE}</p></footer>"#
        )
    }

    #[test]
    fn finds_the_article_and_cleans_it() {
        let a = extract(&page(&article_body()), "https://www.example.com/news/1").unwrap();
        assert_eq!(a.title, "The Real Title");
        assert!(a.content.contains("<h1>Heading</h1>"));
        assert_eq!(a.byline, "Ada Lovelace");
        assert_eq!(a.site, "example.com");
        assert!(a.content.contains("<h1>Heading</h1>"));
        assert_eq!(a.content.matches("<p>").count(), 3, "{}", a.content);
        assert!(
            a.content
                .contains(r#"<a href="https://www.example.com/more">more</a>"#)
        );
        assert!(a.content.contains(
            r#"<img src="https://www.example.com/news/img/a.png" alt="A &quot;pic&quot;">"#
        ));
        for banned in [
            "script", "alert", "onclick", "style=", "Home", "/t/1", "comments",
        ] {
            assert!(
                !a.content.contains(banned),
                "{banned} leaked: {}",
                a.content
            );
        }
        assert!(a.text_len > MIN_TEXT);
        assert_eq!(a.minutes(), 1);
    }

    #[test]
    fn heading_repeating_the_page_title_becomes_the_title() {
        let body = format!(
            "<article><h1>Why Ion</h1><p>{PROSE}</p><p>{PROSE}</p><p>{PROSE}</p></article>"
        );
        let html =
            format!("<html><head><title>Why Ion - Blog</title></head><body>{body}</body></html>");
        let a = extract(&html, "https://example.com/").unwrap();
        assert_eq!(a.title, "Why Ion");
        assert!(!a.content.contains("<h1>"));
    }

    #[test]
    fn short_pages_have_no_article() {
        assert!(extract(&page("<p>Too short.</p>"), "https://example.com/").is_none());
        assert!(extract("", "https://example.com/").is_none());
    }

    #[test]
    fn unsafe_links_are_dropped() {
        let body = format!(
            r#"<div><p>{PROSE} <a href="javascript:evil()">x</a></p><p>{PROSE}</p><p>{PROSE}</p></div>"#
        );
        let a = extract(&page(&body), "https://example.com/").unwrap();
        assert!(a.content.contains("<a>x</a>"));
        assert!(!a.content.contains("javascript"));
    }

    #[test]
    fn furniture_matches_words_not_substrings() {
        let doc = Html::parse_fragment(
            r#"<div class="article-body"></div><div class="nav-links"></div><div id="leadin"></div>"#,
        );
        let divs = Selector::parse("div").unwrap();
        let flags: Vec<bool> = doc.select(&divs).map(is_furniture).collect();
        assert_eq!(flags, [false, true, false]);
    }
}
