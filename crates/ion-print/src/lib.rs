//! Printing in Ion: the decisions behind the print preview.
//!
//! The page is printed to a PDF by the engine, previewed, then either saved
//! or sent to a printer page by page. This crate decides which pages
//! ([`parse_ranges`]), which paper ([`default_paper`], [`PAPERS`]) and what
//! a saved copy is called ([`file_name`]).

/// A paper size Ion offers: its name in `WebEngineView.PrintedPageSizeId`
/// and how it is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paper {
    pub id: &'static str,
    pub label: &'static str,
}

/// The paper sizes in the print preview, most common first.
pub const PAPERS: &[Paper] = &[
    Paper {
        id: "A4",
        label: "A4",
    },
    Paper {
        id: "Letter",
        label: "Letter",
    },
    Paper {
        id: "Legal",
        label: "Legal",
    },
    Paper {
        id: "A3",
        label: "A3",
    },
    Paper {
        id: "A5",
        label: "A5",
    },
    Paper {
        id: "Tabloid",
        label: "Tabloid",
    },
];

/// Letter where it is the norm (the Americas' Letter countries and the
/// Philippines), A4 everywhere else. `locale` is like `"en_US"` or `"de-DE"`.
pub fn default_paper(locale: &str) -> &'static str {
    let region = locale
        .split(['_', '-', '.'])
        .nth(1)
        .unwrap_or("")
        .to_ascii_uppercase();
    match region.as_str() {
        "US" | "CA" | "MX" | "CL" | "CO" | "VE" | "PH" | "GT" | "CR" | "PA" | "DO" | "PR"
        | "SV" | "NI" | "HN" => "Letter",
        _ => "A4",
    }
}

/// The 0-based pages that `text` asks for out of `count`, in order and
/// without repeats. Empty text means every page. Accepts `"1-3, 5, 8-"`,
/// `"-2"` (up to 2) and en dashes; pages past the end are dropped.
pub fn parse_ranges(text: &str, count: u32) -> Result<Vec<u32>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok((0..count).collect());
    }
    let mut pages = Vec::new();
    for part in text.split([',', ';']) {
        let part = part.trim().replace(['–', '—'], "-");
        if part.is_empty() {
            continue;
        }
        let number = |s: &str, default: u32| -> Result<u32, String> {
            let s = s.trim();
            if s.is_empty() {
                return Ok(default);
            }
            match s.parse::<u32>() {
                Ok(0) | Err(_) => Err(format!("\"{part}\" isn't a page number")),
                Ok(n) => Ok(n),
            }
        };
        let (first, last) = match part.split_once('-') {
            Some((a, b)) => (number(a, 1)?, number(b, count.max(1))?),
            None => {
                let n = number(&part, 1)?;
                (n, n)
            }
        };
        if first > last {
            return Err(format!("\"{part}\" runs backwards"));
        }
        for page in first..=last.min(count) {
            if !pages.contains(&(page - 1)) {
                pages.push(page - 1);
            }
        }
    }
    if pages.is_empty() {
        return Err(format!("this page has {count} pages"));
    }
    Ok(pages)
}

/// The file name for a saved copy of the page: its title (or its host, or
/// "Page"), made safe for any file system, with `.pdf`.
pub fn file_name(title: &str, url: &str) -> String {
    let base = if title.trim().is_empty() {
        url.split("://")
            .nth(1)
            .and_then(|rest| rest.split(['/', '?', '#']).next())
            .unwrap_or("")
            .to_owned()
    } else {
        title.to_owned()
    };
    let mut name: String = base
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    name = name.trim_matches('.').trim().to_owned();
    if name.is_empty() {
        name = "Page".to_owned();
    }
    if name.chars().count() > 120 {
        name = name
            .chars()
            .take(120)
            .collect::<String>()
            .trim_end()
            .to_owned();
    }
    format!("{name}.pdf")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_cover_the_usual_forms() {
        assert_eq!(parse_ranges("", 3), Ok(vec![0, 1, 2]));
        assert_eq!(parse_ranges("1-2, 5", 6), Ok(vec![0, 1, 4]));
        assert_eq!(parse_ranges("4-", 5), Ok(vec![3, 4]));
        assert_eq!(parse_ranges("-2", 5), Ok(vec![0, 1]));
        assert_eq!(parse_ranges("2–3; 2", 5), Ok(vec![1, 2]));
        assert_eq!(parse_ranges("3-9", 4), Ok(vec![2, 3]));
    }

    #[test]
    fn bad_ranges_say_why() {
        assert!(parse_ranges("0", 3).is_err());
        assert!(parse_ranges("abc", 3).unwrap_err().contains("abc"));
        assert!(parse_ranges("3-1", 3).unwrap_err().contains("backwards"));
        assert_eq!(
            parse_ranges("7", 3),
            Err("this page has 3 pages".to_owned())
        );
    }

    #[test]
    fn letter_only_where_it_is_used() {
        assert_eq!(default_paper("en_US.UTF-8"), "Letter");
        assert_eq!(default_paper("fr-CA"), "Letter");
        assert_eq!(default_paper("en_GB"), "A4");
        assert_eq!(default_paper("de"), "A4");
        assert_eq!(default_paper(""), "A4");
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(file_name("Cats: a/b?", "https://x.test/"), "Cats a b.pdf");
        assert_eq!(
            file_name("", "https://news.example/a?b"),
            "news.example.pdf"
        );
        assert_eq!(file_name("  ", "about:blank"), "Page.pdf");
        assert_eq!(file_name("...", ""), "Page.pdf");
        assert!(file_name(&"x".repeat(300), "").len() <= 124);
    }
}
