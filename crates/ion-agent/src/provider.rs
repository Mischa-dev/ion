//! The model services Ion Agent can run on, and the presets the setup
//! offers for them.

use std::sync::Arc;

use crate::anthropic::Anthropic;
use crate::model::Model;
use crate::openai::OpenAi;

/// How Ion talks to a service: `ai.provider` and `[agents.<id>] provider`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Any OpenAI-compatible chat completions API.
    OpenAi,
    /// Anthropic's Messages API.
    Anthropic,
}

impl Kind {
    pub fn from_id(id: &str) -> Option<Kind> {
        match id {
            "openai" => Some(Kind::OpenAi),
            "anthropic" => Some(Kind::Anthropic),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Kind::OpenAi => "openai",
            Kind::Anthropic => "anthropic",
        }
    }

    /// A client for `model` at `base_url`.
    pub fn connect(self, base_url: &str, model: &str, key: Option<String>) -> Arc<dyn Model> {
        match self {
            Kind::OpenAi => Arc::new(OpenAi::new(base_url, model, key)),
            Kind::Anthropic => Arc::new(Anthropic::new(base_url, model, key)),
        }
    }
}

/// A service the setup knows how to connect to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    /// Stable id, used by the setup: "anthropic", "ollama", …
    pub id: &'static str,
    pub name: &'static str,
    /// One line on what it is.
    pub blurb: &'static str,
    pub kind: Kind,
    pub base_url: &'static str,
    /// The model picked until the person chooses another.
    pub model: &'static str,
    /// Where Ion looks for the key when none is saved; empty for services
    /// that take none.
    pub key_env: &'static str,
    /// Where to get a key, for the setup's link.
    pub key_page: &'static str,
    /// Runs on the person's own machine: nothing leaves it.
    pub local: bool,
}

impl Preset {
    pub fn needs_key(&self) -> bool {
        !self.key_env.is_empty()
    }
}

pub const PRESETS: [Preset; 6] = [
    Preset {
        id: "anthropic",
        name: "Claude",
        blurb: "Anthropic's models. Strong at reading pages and acting carefully.",
        kind: Kind::Anthropic,
        base_url: "https://api.anthropic.com/v1",
        model: "claude-opus-5-5",
        key_env: "ANTHROPIC_API_KEY",
        key_page: "https://console.anthropic.com/settings/keys",
        local: false,
    },
    Preset {
        id: "openai",
        name: "OpenAI",
        blurb: "GPT models through the OpenAI API.",
        kind: Kind::OpenAi,
        base_url: "https://api.openai.com/v1",
        model: "gpt-5-mini",
        key_env: "OPENAI_API_KEY",
        key_page: "https://platform.openai.com/api-keys",
        local: false,
    },
    Preset {
        id: "openrouter",
        name: "OpenRouter",
        blurb: "Many providers' models with one key.",
        kind: Kind::OpenAi,
        base_url: "https://openrouter.ai/api/v1",
        model: "openrouter/auto",
        key_env: "OPENROUTER_API_KEY",
        key_page: "https://openrouter.ai/keys",
        local: false,
    },
    Preset {
        id: "ollama",
        name: "Ollama",
        blurb: "A model running on this computer. Nothing leaves it.",
        kind: Kind::OpenAi,
        base_url: "http://127.0.0.1:11434/v1",
        model: "qwen3:8b",
        key_env: "",
        key_page: "https://ollama.com/library",
        local: true,
    },
    Preset {
        id: "lmstudio",
        name: "LM Studio",
        blurb: "LM Studio's local server on this computer.",
        kind: Kind::OpenAi,
        base_url: "http://127.0.0.1:1234/v1",
        model: "",
        key_env: "",
        key_page: "https://lmstudio.ai",
        local: true,
    },
    Preset {
        id: "custom",
        name: "Other",
        blurb: "Any OpenAI-compatible service: llama.cpp, vLLM, a company gateway.",
        kind: Kind::OpenAi,
        base_url: "",
        model: "",
        key_env: "",
        key_page: "",
        local: false,
    },
];

pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// The preset that `kind` at `base_url` looks like, for showing the
/// current setup; "custom" when none does.
pub fn preset_for(kind: Kind, base_url: &str) -> &'static Preset {
    let base = base_url.trim().trim_end_matches('/');
    PRESETS
        .iter()
        .find(|p| p.id != "custom" && p.kind == kind && p.base_url == base)
        .unwrap_or(&PRESETS[PRESETS.len() - 1])
}

/// The keyring entry a service's key is saved under: its kind and host, so
/// one key serves every agent on the same service and a key is never sent
/// to a different host than the one it was saved for.
pub fn key_account(kind: Kind, base_url: &str) -> Option<String> {
    let url = url::Url::parse(base_url.trim()).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    Some(match url.port() {
        Some(port) => format!("{}:{host}:{port}", kind.id()),
        None => format!("{}:{host}", kind.id()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_well_formed() {
        for p in PRESETS {
            assert_eq!(preset(p.id), Some(&p));
            if p.id != "custom" {
                assert!(key_account(p.kind, p.base_url).is_some(), "{}", p.id);
                assert_eq!(preset_for(p.kind, p.base_url).id, p.id);
            }
            assert_eq!(p.local, !p.needs_key() && p.id != "custom");
        }
        assert_eq!(Kind::from_id("anthropic"), Some(Kind::Anthropic));
        assert_eq!(Kind::from_id("openai").map(Kind::id), Some("openai"));
        assert_eq!(Kind::from_id("gemini"), None);
    }

    #[test]
    fn recognises_presets_by_address() {
        assert_eq!(
            preset_for(Kind::Anthropic, "https://api.anthropic.com/v1/").id,
            "anthropic"
        );
        assert_eq!(
            preset_for(Kind::OpenAi, "https://api.openai.com/v1").id,
            "openai"
        );
        assert_eq!(
            preset_for(Kind::OpenAi, "https://llm.corp.example/v1").id,
            "custom"
        );
        // The same address spoken to the other way isn't that preset.
        assert_eq!(
            preset_for(Kind::OpenAi, "https://api.anthropic.com/v1").id,
            "custom"
        );
    }

    #[test]
    fn keys_are_saved_per_service_host() {
        assert_eq!(
            key_account(Kind::Anthropic, "https://api.anthropic.com/v1").as_deref(),
            Some("anthropic:api.anthropic.com")
        );
        assert_eq!(
            key_account(Kind::OpenAi, "http://127.0.0.1:8080/v1").as_deref(),
            Some("openai:127.0.0.1:8080")
        );
        assert_eq!(
            key_account(Kind::OpenAi, "https://API.OpenAI.com/v1"),
            key_account(Kind::OpenAi, "https://api.openai.com/v1/")
        );
        assert_eq!(key_account(Kind::OpenAi, "not a url"), None);
    }
}
