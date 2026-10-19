//! Starting a task from the address bar: `@ion what is this page about?`
//! asks the agent with id `ion`; `!ai …` is the same as `@ion …`.

use ion_safety::policy::is_valid_id;

/// The default agent, used by `!ai` and a bare question to Ion Agent.
pub const DEFAULT_AGENT: &str = "ion";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub agent: String,
    pub question: String,
}

/// A ready-made question offered under `@ion `, so the common asks are a
/// keypress away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickAsk {
    /// What the suggestion says: "Summarize this page".
    pub title: &'static str,
    /// What is asked.
    pub question: &'static str,
}

pub const QUICK_ASKS: [QuickAsk; 5] = [
    QuickAsk {
        title: "Summarize this page",
        question: "Summarize this page.",
    },
    QuickAsk {
        title: "Key points",
        question: "What are the key points of this page?",
    },
    QuickAsk {
        title: "Compare my open tabs",
        question: "Compare my open tabs: what each offers and how they differ.",
    },
    QuickAsk {
        title: "Sum up my open tabs",
        question: "Sum up each of my open tabs in a line.",
    },
    QuickAsk {
        title: "What should I know before I act on this?",
        question: "What should I know before I act on this page? Note any catches, costs or deadlines.",
    },
];

/// The quick asks for what has been typed after the agent: all of them for
/// nothing yet, else those whose title or question starts with it.
pub fn quick_asks(typed: &str) -> Vec<QuickAsk> {
    let typed = typed.trim().to_lowercase();
    QUICK_ASKS
        .into_iter()
        .filter(|q| {
            typed.is_empty()
                || (q.title.to_lowercase().starts_with(&typed)
                    || q.question.to_lowercase().starts_with(&typed))
                    && q.question.to_lowercase() != typed
        })
        .collect()
}

/// The agent being addressed and what follows it, even when that is
/// nothing yet: `"@ion "` gives `("ion", "")`. `None` while the agent's
/// name is still being typed or for other input.
pub fn addressed(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    let (agent, rest) = if let Some(rest) = input.strip_prefix("!ai") {
        (DEFAULT_AGENT, rest)
    } else {
        let rest = input.strip_prefix('@')?;
        let end = rest.find(char::is_whitespace)?;
        (&rest[..end], &rest[end..])
    };
    (rest.starts_with(char::is_whitespace) && is_valid_id(agent)).then(|| (agent, rest.trim()))
}

/// The agent and question in address-bar input, or `None` when the input
/// isn't addressed to an agent or has no question yet.
pub fn parse(input: &str) -> Option<Invocation> {
    let input = input.trim_start();
    let (agent, rest) = if let Some(rest) = input.strip_prefix("!ai") {
        (DEFAULT_AGENT, rest)
    } else {
        let rest = input.strip_prefix('@')?;
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        (&rest[..end], &rest[end..])
    };
    // `!aisle` is not `!ai`; the question must be separated by a space.
    if !(rest.is_empty() || rest.starts_with(char::is_whitespace)) || !is_valid_id(agent) {
        return None;
    }
    let question = rest.trim();
    (!question.is_empty()).then(|| Invocation {
        agent: agent.to_owned(),
        question: question.to_owned(),
    })
}

/// The agent id being typed, for suggestions: `"@cl"` gives `"cl"`, and
/// `"@"` gives `""`. `None` once a question follows or for other input.
pub fn agent_prefix(input: &str) -> Option<&str> {
    let rest = input.trim_start().strip_prefix('@')?;
    (!rest.contains(char::is_whitespace)).then_some(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(agent: &str, question: &str) -> Option<Invocation> {
        Some(Invocation {
            agent: agent.into(),
            question: question.into(),
        })
    }

    #[test]
    fn quick_asks_follow_what_is_typed() {
        assert_eq!(addressed("@ion "), Some(("ion", "")));
        assert_eq!(addressed("!ai  key"), Some(("ion", "key")));
        assert_eq!(addressed("@io"), None);
        assert_eq!(addressed("!aisle x"), None);
        assert_eq!(addressed("hello"), None);
        assert_eq!(quick_asks("").len(), QUICK_ASKS.len());
        let titles: Vec<&str> = quick_asks("sum").iter().map(|q| q.title).collect();
        assert_eq!(titles, ["Summarize this page", "Sum up my open tabs"]);
        assert_eq!(quick_asks("Compare").len(), 1);
        // Typed out in full, the plain ask covers it.
        assert!(quick_asks("Summarize this page.").is_empty());
        assert!(quick_asks("why is the sky blue").is_empty());
    }

    #[test]
    fn at_names_the_agent() {
        assert_eq!(
            parse("@ion what is this page about?"),
            ask("ion", "what is this page about?")
        );
        assert_eq!(
            parse("  @research   find  sources "),
            ask("research", "find  sources")
        );
        assert_eq!(parse("@claude-code fix it"), ask("claude-code", "fix it"));
    }

    #[test]
    fn bang_ai_asks_ion_agent() {
        assert_eq!(parse("!ai summarize"), ask("ion", "summarize"));
        assert_eq!(parse("!aisle 5"), None);
        assert_eq!(parse("!ai"), None);
        assert_eq!(parse("!ai   "), None);
    }

    #[test]
    fn needs_a_question_and_a_valid_id() {
        assert_eq!(parse("@ion"), None);
        assert_eq!(parse("@ion "), None);
        assert_eq!(parse("@"), None);
        assert_eq!(parse("@ question"), None);
        assert_eq!(parse("@bad/id question"), None);
        assert_eq!(parse("ion question"), None);
        assert_eq!(parse("https://example.com/@ion"), None);
    }

    #[test]
    fn agent_prefix_while_typing() {
        assert_eq!(agent_prefix("@"), Some(""));
        assert_eq!(agent_prefix("@cl"), Some("cl"));
        assert_eq!(agent_prefix("@ion what"), None);
        assert_eq!(agent_prefix("ion"), None);
    }
}
