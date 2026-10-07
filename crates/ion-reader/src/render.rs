//! The reader page itself.

use crate::Article;
use crate::extract::escape_into;

/// Colors and type for the reader page, taken from Ion's theme. Colors are
/// CSS color strings (`#rrggbb`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    pub background: String,
    pub text: String,
    pub muted: String,
    pub accent: String,
    pub surface: String,
    pub border: String,
    /// Body font size in CSS pixels.
    pub font_size: u32,
    /// Use a serif face for the body text.
    pub serif: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: "#1e1e2e".into(),
            text: "#cdd6f4".into(),
            muted: "#a6adc8".into(),
            accent: "#89b4fa".into(),
            surface: "#313244".into(),
            border: "#45475a".into(),
            font_size: 19,
            serif: true,
        }
    }
}

/// Only CSS color syntax gets into the style sheet.
fn css_color(value: &str) -> &str {
    let ok = !value.is_empty()
        && value.len() <= 32
        && value.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '#' | '(' | ')' | ',' | '.' | ' ' | '%')
        });
    if ok { value } else { "inherit" }
}

/// A complete HTML page showing `article` in `style`. The page allows no
/// scripts, so nothing from the original page can run in it.
pub fn render(article: &Article, style: &Style) -> String {
    let mut title = String::new();
    escape_into(&article.title, &mut title);
    let mut meta = String::new();
    let parts: Vec<&str> = [article.site.as_str(), article.byline.as_str()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    escape_into(&parts.join(" · "), &mut meta);
    let minutes = article.minutes();
    let reading = format!("{minutes} min read");
    let meta = if meta.is_empty() {
        reading
    } else {
        format!("{meta} · {reading}")
    };
    let font = if style.serif {
        r#"Charter, "Bitstream Charter", "Iowan Old Style", Georgia, "Noto Serif", serif"#
    } else {
        r#"system-ui, -apple-system, "Segoe UI", "Noto Sans", sans-serif"#
    };
    let (bg, text, muted, accent, surface, border) = (
        css_color(&style.background),
        css_color(&style.text),
        css_color(&style.muted),
        css_color(&style.accent),
        css_color(&style.surface),
        css_color(&style.border),
    );
    let size = style.font_size.clamp(12, 32);
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src * data:; style-src 'unsafe-inline'; media-src *">
<meta name="viewport" content="width=device-width">
<title>{title}</title>
<style>
:root {{ color-scheme: light dark; }}
html {{ background: {bg}; }}
body {{ margin: 0 auto; max-width: 42em; padding: 3em 1.5em 6em; color: {text};
  font: {size}px/1.65 {font}; }}
header {{ margin-bottom: 2.2em; padding-bottom: 1.2em; border-bottom: 1px solid {border}; }}
header h1 {{ font-size: 1.9em; line-height: 1.2; margin: 0 0 .4em; }}
.meta {{ color: {muted}; font: .8em/1.4 system-ui, sans-serif; }}
h1, h2, h3, h4, h5, h6 {{ line-height: 1.3; margin: 1.6em 0 .5em; }}
a {{ color: {accent}; text-decoration-thickness: 1px; text-underline-offset: 2px; }}
img, video, picture {{ max-width: 100%; height: auto; border-radius: 6px; }}
figure {{ margin: 1.5em 0; }}
figcaption {{ color: {muted}; font-size: .8em; margin-top: .4em; }}
blockquote {{ margin: 1.2em 0; padding: 0 1em; border-left: 3px solid {accent}; color: {muted}; }}
pre, code, kbd {{ font-family: ui-monospace, "JetBrains Mono", Menlo, monospace; font-size: .85em; }}
pre {{ background: {surface}; padding: 1em; border-radius: 6px; overflow-x: auto; line-height: 1.45; }}
:not(pre) > code {{ background: {surface}; padding: .1em .3em; border-radius: 4px; }}
table {{ border-collapse: collapse; margin: 1em 0; font-size: .9em; }}
td, th {{ border: 1px solid {border}; padding: .3em .6em; }}
hr {{ border: 0; border-top: 1px solid {border}; margin: 2em 0; }}
::selection {{ background: {accent}; color: {bg}; }}
</style></head>
<body><header><h1>{title}</h1><div class="meta">{meta}</div></header>
<main>
{content}
</main></body></html>
"#,
        content = article.content,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn article() -> Article {
        Article {
            title: "Cats & <Dogs>".into(),
            byline: "Ada".into(),
            site: "example.com".into(),
            content: "<p>Body</p>".into(),
            text_len: 3000,
        }
    }

    #[test]
    fn page_is_escaped_themed_and_scriptless() {
        let html = render(&article(), &Style::default());
        assert!(html.contains("<title>Cats &amp; &lt;Dogs&gt;</title>"));
        assert!(html.contains("example.com · Ada · 3 min read"));
        assert!(html.contains("background: #1e1e2e"));
        assert!(html.contains("default-src 'none'"));
        assert!(html.contains("<p>Body</p>"));
    }

    #[test]
    fn hostile_colors_are_rejected() {
        let style = Style {
            text: "red; } body { display:none".into(),
            ..Style::default()
        };
        let html = render(&article(), &style);
        assert!(html.contains("color: inherit"));
        assert!(!html.contains("display:none"));
        assert_eq!(css_color("rgba(1, 2, 3, 0.5)"), "rgba(1, 2, 3, 0.5)");
    }
}
