//! Answers come back as Markdown; this turns the little of it a short
//! answer uses into HTML for Qt's rich text: paragraphs, bullet and
//! numbered lists, `code` and **bold**. Everything else stays as text, and
//! all text is escaped, so a model can't inject markup.

pub fn to_html(markdown: &str) -> String {
    let mut html = String::new();
    let mut list: Option<&str> = None;
    let mut paragraph: Vec<String> = Vec::new();

    let flush = |html: &mut String, paragraph: &mut Vec<String>| {
        if !paragraph.is_empty() {
            html.push_str("<p>");
            html.push_str(&inline(&paragraph.join(" ")));
            html.push_str("</p>");
            paragraph.clear();
        }
    };

    for line in markdown.lines() {
        let trimmed = line.trim();
        let item = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .map(|rest| ("ul", rest))
            .or_else(|| numbered(trimmed).map(|rest| ("ol", rest)));
        match item {
            Some((kind, rest)) => {
                flush(&mut html, &mut paragraph);
                if list != Some(kind) {
                    if let Some(open) = list {
                        html.push_str(&format!("</{open}>"));
                    }
                    html.push_str(&format!("<{kind}>"));
                    list = Some(kind);
                }
                html.push_str(&format!("<li>{}</li>", inline(rest)));
            }
            None => {
                if let Some(open) = list.take() {
                    html.push_str(&format!("</{open}>"));
                }
                if trimmed.is_empty() {
                    flush(&mut html, &mut paragraph);
                } else {
                    paragraph.push(trimmed.trim_start_matches('#').trim().to_owned());
                }
            }
        }
    }
    if let Some(open) = list {
        html.push_str(&format!("</{open}>"));
    }
    flush(&mut html, &mut paragraph);
    html
}

/// The text after "1. " or "12) ".
fn numbered(line: &str) -> Option<&str> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let rest = &line[digits..];
    rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") "))
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// `code` and **bold** within one line; unmatched markers stay as text.
fn inline(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let tick = rest.find('`');
        let stars = rest.find("**");
        let next = match (tick, stars) {
            (Some(t), Some(s)) => Some(t.min(s)),
            (t, s) => t.or(s),
        };
        let Some(start) = next else {
            out.push_str(&escape(rest));
            break;
        };
        out.push_str(&escape(&rest[..start]));
        let (marker, open, close) = if rest[start..].starts_with('`') {
            ("`", "<code>", "</code>")
        } else {
            ("**", "<b>", "</b>")
        };
        let body = &rest[start + marker.len()..];
        match body.find(marker) {
            Some(end) if end > 0 => {
                out.push_str(open);
                out.push_str(&escape(&body[..end]));
                out.push_str(close);
                rest = &body[end + marker.len()..];
            }
            _ => {
                out.push_str(&escape(marker));
                rest = body;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_code_and_bold() {
        assert_eq!(
            to_html("Set `QT_QPA_PLATFORM=wayland` **first**.\nThen run it.\n\nDone."),
            "<p>Set <code>QT_QPA_PLATFORM=wayland</code> <b>first</b>. Then run it.</p><p>Done.</p>"
        );
    }

    #[test]
    fn lists() {
        assert_eq!(
            to_html("Two ways:\n- one\n- two `x`\n1. first\n2) second\nAfter."),
            "<p>Two ways:</p><ul><li>one</li><li>two <code>x</code></li></ul>\
             <ol><li>first</li><li>second</li></ol><p>After.</p>"
        );
    }

    #[test]
    fn markup_is_escaped() {
        assert_eq!(
            to_html("<img src=x onerror=alert(1)> & `<b>`"),
            "<p>&lt;img src=x onerror=alert(1)&gt; &amp; <code>&lt;b&gt;</code></p>"
        );
        assert_eq!(to_html("a ` b ** c"), "<p>a ` b ** c</p>");
        assert_eq!(to_html("## Heading"), "<p>Heading</p>");
    }
}
