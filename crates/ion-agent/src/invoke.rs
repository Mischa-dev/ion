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
