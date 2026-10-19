use crate::{Context, Control, Group, Section, Setting, choices};

pub fn section(_: &Context) -> Section {
    let text = |placeholder: &str| Control::Text {
        placeholder: placeholder.into(),
    };
    Section::new(
        "agent",
        "Ion Agent",
        "✦",
        "Ask about a page, or have Ion do things for you, from the address bar.",
        vec![
            Group::new(
                "",
                vec![
                    Setting::new("ai.enable", "Ion Agent", Control::Toggle)
                        .help("Type @ion in the address bar to ask. Nothing is sent anywhere while this is off.")
                        .keywords("ai assistant llm chat @ion"),
                    Setting::new(
                        "ai.setUp",
                        "Set up",
                        Control::Action {
                            action: "set-up-agent".into(),
                            button: "Set up Ion Agent".into(),
                        },
                    )
                    .help("Pick Claude, OpenAI, OpenRouter, Ollama or another service, paste a key and choose a model.")
                    .keywords("api key claude openai ollama provider keyring"),
                ],
            ),
            Group::new(
                "Model",
                vec![
                    Setting::new(
                        "ai.provider",
                        "Kind of service",
                        Control::Choice {
                            options: choices(&[
                                ("openai", "OpenAI-compatible"),
                                ("anthropic", "Anthropic (Claude)"),
                            ]),
                        },
                    )
                    .keywords("provider claude anthropic openai api")
                    .requires("ai.enable"),
                    Setting::new("ai.model", "Model", text("gpt-5-mini"))
                        .keywords("llm openai ollama")
                        .requires("ai.enable"),
                    Setting::new("ai.baseUrl", "Service address", text("https://api.openai.com/v1"))
                        .help("Up to and including /v1, such as http://127.0.0.1:11434/v1 for Ollama.")
                        .keywords("endpoint url local ollama openrouter llama.cpp")
                        .requires("ai.enable"),
                    Setting::new("ai.apiKeyEnv", "API key variable", text("OPENAI_API_KEY"))
                        .help("Used when no key is saved in the system keyring (Set up Ion Agent saves it there). Empty for local models.")
                        .keywords("token secret environment")
                        .requires("ai.enable"),
                ],
            ),
            Group::new(
                "History",
                vec![
                    Setting::new(
                        "ai.keepConversationsDays",
                        "Keep conversations for",
                        Control::Number {
                            min: 0.0,
                            max: 365.0,
                            step: 1.0,
                            unit: "days".into(),
                            slider: false,
                        },
                    )
                    .help("To reopen them from the agent sidebar. Page contents are never kept.")
                    .requires("ai.enable"),
                    Setting::new(
                        "safety.auditRetentionDays",
                        "Keep the activity log for",
                        Control::Number {
                            min: 1.0,
                            max: 365.0,
                            step: 1.0,
                            unit: "days".into(),
                            slider: false,
                        },
                    )
                    .help("Every action an agent took, and what you allowed.")
                    .keywords("audit safety log"),
                ],
            ),
            Group::new(
                "Outside agents",
                vec![
                    Setting::new(
                        "agents.mcp.enable",
                        "Let agents on this computer use Ion",
                        Control::Toggle,
                    )
                    .help("Serves Ion's browser tools over MCP on localhost, with the same permission prompts.")
                    .keywords("mcp claude code codex server"),
                ],
            ),
        ],
    )
}
