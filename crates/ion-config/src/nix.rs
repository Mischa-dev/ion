//! "Promote to Nix": render settings as a `programs.ion` snippet that can be
//! pasted into a home-manager config. TOML keys and Nix option names are the
//! same, so this is a direct translation.

use toml::{Table, Value};

/// Render `table` as `programs.ion = { … };`, one line per setting.
pub fn snippet(table: &Table) -> String {
    let mut lines = Vec::new();
    leaves(table, &mut Vec::new(), &mut lines);
    if lines.is_empty() {
        return "programs.ion = { };\n".to_owned();
    }
    let mut out = String::from("programs.ion = {\n");
    for line in lines {
        out.push_str("  ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("};\n");
    out
}

/// Collect `a.b.c = value;` for every non-table value under `table`.
fn leaves(table: &Table, prefix: &mut Vec<String>, out: &mut Vec<String>) {
    for (key, value) in table {
        prefix.push(attr_name(key));
        match value {
            Value::Table(inner) if !inner.is_empty() => leaves(inner, prefix, out),
            _ => out.push(format!("{} = {};", prefix.join("."), expr(value))),
        }
        prefix.pop();
    }
}

/// A Nix expression for a TOML value.
pub fn expr(value: &Value) -> String {
    match value {
        Value::String(s) => string(s),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) if f.is_finite() && f.fract() == 0.0 => format!("{f:.1}"),
        Value::Float(f) => f.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Datetime(d) => string(&d.to_string()),
        Value::Array(items) if items.is_empty() => "[ ]".to_owned(),
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(expr).collect();
            format!("[ {} ]", items.join(" "))
        }
        Value::Table(t) if t.is_empty() => "{ }".to_owned(),
        Value::Table(t) => {
            let fields: Vec<String> = t
                .iter()
                .map(|(k, v)| format!("{} = {};", attr_name(k), expr(v)))
                .collect();
            format!("{{ {} }}", fields.join(" "))
        }
    }
}

fn string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '$' if chars.peek() == Some(&'{') => out.push_str("\\$"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// An attribute name, quoted unless it is a plain identifier.
fn attr_name(key: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "if", "then", "else", "assert", "with", "let", "in", "rec", "inherit", "or",
    ];
    let mut chars = key.chars();
    let plain = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '\'' | '-'))
        && !KEYWORDS.contains(&key);
    if plain { key.to_owned() } else { string(key) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_dotted_settings() {
        let table: Table = r#"
            bangs = { gh = "https://github.com/search?q={}", "c++" = "https://cppreference.com/?q={}" }
            [ui]
            cornerRadius = 12
            animations = { enable = false, speed = 1.5 }
            [adblock]
            lists = ["easylist"]
            [theme]
            source = "dms"
            "#
        .parse()
        .unwrap();
        assert_eq!(
            snippet(&table),
            r#"programs.ion = {
  adblock.lists = [ "easylist" ];
  bangs."c++" = "https://cppreference.com/?q={}";
  bangs.gh = "https://github.com/search?q={}";
  theme.source = "dms";
  ui.animations.enable = false;
  ui.animations.speed = 1.5;
  ui.cornerRadius = 12;
};
"#
        );
    }

    #[test]
    fn empty_is_an_empty_set() {
        assert_eq!(snippet(&Table::new()), "programs.ion = { };\n");
    }

    #[test]
    fn escapes_strings_and_keeps_floats_floats() {
        assert_eq!(
            expr(&Value::String("a\"b\\${x}$y".into())),
            r#""a\"b\\\${x}$y""#
        );
        assert_eq!(expr(&Value::Float(2.0)), "2.0");
        assert_eq!(attr_name("let"), "\"let\"");
        assert_eq!(attr_name("restore-session"), "restore-session");
    }
}
