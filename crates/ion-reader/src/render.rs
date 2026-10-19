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
    /// Widest the column of text gets, in `em`.
    pub width_em: u32,
}

/// Reader page colors of their own, for `[reader] theme`: `"light"`,
/// `"sepia"` or `"dark"`. `None` for `"match"` (Ion's theme) and anything
/// else.
pub fn preset(theme: &str) -> Option<[&'static str; 6]> {
    // background, text, muted, accent, surface, border
    Some(match theme {
        "light" => [
            "#ffffff", "#1f2328", "#59636e", "#0b63ce", "#f3f4f6", "#d8dee4",
        ],
        "sepia" => [
            "#f4ecd8", "#433422", "#7a6a53", "#9a5b13", "#ebe0c6", "#d9cba8",
        ],
        "dark" => [
            "#16171a", "#d6d6d6", "#9a9a9a", "#7fb0ff", "#24262b", "#33363c",
        ],
        _ => return None,
    })
}

impl Style {
    /// Applies a [`preset`]'s colors; unknown names leave the style as is.
    pub fn with_preset(mut self, theme: &str) -> Self {
        if let Some([bg, text, muted, accent, surface, border]) = preset(theme) {
            self.background = bg.into();
            self.text = text.into();
            self.muted = muted.into();
            self.accent = accent.into();
            self.surface = surface.into();
            self.border = border.into();
        }
        self
    }

    /// The page's CSS custom properties, cleaned for the style sheet.
    fn variables(&self) -> [(&'static str, String); 9] {
        let font = if self.serif { SERIF } else { SANS };
        [
            ("--bg", css_color(&self.background).to_owned()),
            ("--text", css_color(&self.text).to_owned()),
            ("--muted", css_color(&self.muted).to_owned()),
            ("--accent", css_color(&self.accent).to_owned()),
            ("--surface", css_color(&self.surface).to_owned()),
            ("--border", css_color(&self.border).to_owned()),
            ("--size", format!("{}px", self.font_size.clamp(12, 32))),
            ("--width", format!("{}em", self.width_em.clamp(28, 60))),
            ("--font", font.to_owned()),
        ]
    }
}

const SERIF: &str =
    r#"Charter, "Bitstream Charter", "Iowan Old Style", Georgia, "Noto Serif", serif"#;
const SANS: &str = r#"system-ui, -apple-system, "Segoe UI", "Noto Sans", sans-serif"#;

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
            width_em: 38,
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

/// Script (Ion's world) restyling an open reader page to `style` in place,
/// keeping its scroll position.
pub fn style_script(style: &Style) -> String {
    let mut script = String::from("(() => { const s = document.documentElement.style;");
    for (name, value) in style.variables() {
        // Values are already limited to CSS-safe characters; quotes inside
        // font lists are double, so single quotes delimit them here.
        script.push_str(&format!(
            " s.setProperty('{name}', '{}');",
            value.replace('\'', "")
        ));
    }
    script.push_str(" })();");
    script
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
    let variables: String = style
        .variables()
        .iter()
        .map(|(name, value)| format!("{name}: {value}; "))
        .collect();
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src * data:; style-src 'unsafe-inline'; media-src *">
<meta name="viewport" content="width=device-width">
<title>{title}</title>
<style>
:root {{ color-scheme: light dark; {variables}}}
html {{ background: var(--bg); transition: background .2s; }}
body {{ margin: 0 auto; max-width: var(--width); padding: 3em 1.5em 6em; color: var(--text);
  font: var(--size)/1.65 var(--font); }}
.progress {{ position: fixed; top: 0; left: 0; right: 0; height: 3px; background: var(--accent);
  transform-origin: 0 50%; transform: scaleX(0); animation: progress linear both;
  animation-timeline: scroll(root); }}
@keyframes progress {{ to {{ transform: scaleX(1); }} }}
header {{ margin-bottom: 2.2em; padding-bottom: 1.2em; border-bottom: 1px solid var(--border); }}
header h1 {{ font-size: 1.9em; line-height: 1.2; margin: 0 0 .4em; }}
.meta {{ color: var(--muted); font: .8em/1.4 system-ui, sans-serif; }}
h1, h2, h3, h4, h5, h6 {{ line-height: 1.3; margin: 1.6em 0 .5em; }}
a {{ color: var(--accent); text-decoration-thickness: 1px; text-underline-offset: 2px; }}
img, video, picture {{ max-width: 100%; height: auto; border-radius: 6px; }}
figure {{ margin: 1.5em 0; }}
figcaption {{ color: var(--muted); font-size: .8em; margin-top: .4em; }}
blockquote {{ margin: 1.2em 0; padding: 0 1em; border-left: 3px solid var(--accent); color: var(--muted); }}
pre, code, kbd {{ font-family: ui-monospace, "JetBrains Mono", Menlo, monospace; font-size: .85em; }}
pre {{ background: var(--surface); padding: 1em; border-radius: 6px; overflow-x: auto; line-height: 1.45; }}
:not(pre) > code {{ background: var(--surface); padding: .1em .3em; border-radius: 4px; }}
table {{ border-collapse: collapse; margin: 1em 0; font-size: .9em; }}
td, th {{ border: 1px solid var(--border); padding: .3em .6em; }}
hr {{ border: 0; border-top: 1px solid var(--border); margin: 2em 0; }}
::selection {{ background: var(--accent); color: var(--bg); }}
</style></head>
<body><div class="progress"></div><header><h1>{title}</h1><div class="meta">{meta}</div></header>
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
        assert!(html.contains("--bg: #1e1e2e;"));
        assert!(html.contains("--width: 38em;"));
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
        assert!(html.contains("--text: inherit;"));
        assert!(!html.contains("display:none"));
        assert_eq!(css_color("rgba(1, 2, 3, 0.5)"), "rgba(1, 2, 3, 0.5)");
    }

    #[test]
    fn presets_and_live_restyling() {
        let style = Style {
            font_size: 99,
            serif: false,
            ..Style::default()
        }
        .with_preset("sepia");
        assert_eq!(style.background, "#f4ecd8");
        assert_eq!(Style::default().with_preset("match"), Style::default());
        let script = style_script(&style);
        assert!(script.contains("s.setProperty('--bg', '#f4ecd8');"));
        assert!(script.contains("s.setProperty('--size', '32px');"));
        assert!(script.contains("system-ui"));
    }
}
