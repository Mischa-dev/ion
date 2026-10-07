//! uBlock Origin's `!#if` / `!#else` / `!#endif` preprocessor directives.
//!
//! adblock-rust treats them as comments, which would keep every branch's
//! rules, including Firefox- or Safari-only exceptions. This keeps only the
//! lines meant for Ion: a Chromium-based browser understanding uBlock syntax.

/// Tokens that hold for Ion. Every other token (`env_firefox`, `env_mobile`,
/// `cap_html_filtering`, `adguard`, …) is false.
const TRUE_TOKENS: &[&str] = &["env_chromium", "ext_ublock"];

/// Drop the lines of `text` that sit in branches not meant for Ion.
pub fn preprocess(text: &str) -> String {
    if !text.contains("!#if") {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    // For each open `!#if`: whether its current branch is active.
    let mut stack: Vec<bool> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(expr) = trimmed.strip_prefix("!#if ") {
            stack.push(evaluate(expr));
        } else if trimmed == "!#else" {
            if let Some(top) = stack.last_mut() {
                *top = !*top;
            }
        } else if trimmed == "!#endif" {
            stack.pop();
        } else if stack.iter().all(|&active| active) {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Evaluate a directive condition: tokens joined by `!`, `&&`, `||` and
/// parentheses. Anything malformed is false, so its section is dropped.
fn evaluate(expr: &str) -> bool {
    let tokens = tokenize(expr);
    let mut parser = Parser {
        tokens: &tokens,
        pos: 0,
    };
    let value = parser.or();
    value
        .filter(|_| parser.pos == tokens.len())
        .unwrap_or(false)
}

fn tokenize(expr: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut rest = expr.trim();
    while !rest.is_empty() {
        let len = if rest.starts_with("&&") || rest.starts_with("||") {
            2
        } else if rest.starts_with(['!', '(', ')']) {
            1
        } else {
            rest.find(|c: char| c.is_whitespace() || "!()&|".contains(c))
                .unwrap_or(rest.len())
                .max(1)
        };
        tokens.push(&rest[..len]);
        rest = rest[len..].trim_start();
    }
    tokens
}

struct Parser<'a> {
    tokens: &'a [&'a str],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.tokens.get(self.pos).copied()
    }

    fn or(&mut self) -> Option<bool> {
        let mut value = self.and()?;
        while self.peek() == Some("||") {
            self.pos += 1;
            value |= self.and()?;
        }
        Some(value)
    }

    fn and(&mut self) -> Option<bool> {
        let mut value = self.unary()?;
        while self.peek() == Some("&&") {
            self.pos += 1;
            value &= self.unary()?;
        }
        Some(value)
    }

    fn unary(&mut self) -> Option<bool> {
        let token = self.peek()?;
        self.pos += 1;
        match token {
            "!" => self.unary().map(|v| !v),
            "(" => {
                let value = self.or()?;
                if self.peek() != Some(")") {
                    return None;
                }
                self.pos += 1;
                Some(value)
            }
            "&&" | "||" | ")" => None,
            name => Some(TRUE_TOKENS.contains(&name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_conditions() {
        assert!(evaluate("env_chromium"));
        assert!(!evaluate("env_firefox"));
        assert!(evaluate("!env_safari"));
        assert!(evaluate("env_firefox || env_chromium"));
        assert!(!evaluate("env_chromium && env_mobile"));
        assert!(evaluate("!(env_firefox || env_mobile) && ext_ublock"));
        assert!(!evaluate("(env_chromium"));
        assert!(!evaluate(""));
    }

    #[test]
    fn keeps_only_branches_for_ion() {
        let list = "\
a
!#if env_firefox
firefox-only
!#if env_mobile
nested
!#endif
!#else
not-firefox
!#endif
!#if !env_chromium
other
!#endif
b
";
        assert_eq!(preprocess(list), "a\nnot-firefox\nb\n");
    }

    #[test]
    fn leaves_plain_lists_alone() {
        let list = "! Title: x\n||ads.example^";
        assert_eq!(preprocess(list), list);
    }
}
