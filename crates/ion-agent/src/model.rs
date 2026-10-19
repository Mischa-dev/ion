//! The conversation with a model, independent of any one service's format.

use crate::tool::Tool;

/// A call the model asked for: `name` is a [`Tool`] name and `arguments`
/// the JSON text it sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    /// Provider-assigned identifier used when returning the tool result.
    pub id: String,
    /// Browser tool name requested by the model.
    pub name: String,
    /// JSON object encoded as text, following the provider protocol.
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    System(String),
    User(String),
    Assistant {
        text: String,
        calls: Vec<ToolCall>,
        /// The service's own record of the turn (JSON), for services that
        /// need it sent back unchanged (Anthropic's thinking blocks). Empty
        /// for others.
        raw: String,
    },
    /// A tool's result, answering the call with `call_id`.
    Tool {
        call_id: String,
        content: String,
    },
}

/// One turn from the model: some text, and the tools it wants run next.
/// No calls means the text is the answer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reply {
    /// Natural-language content produced during this turn.
    pub text: String,
    /// Browser operations the model wants the runtime to consider next.
    pub calls: Vec<ToolCall>,
    /// See [`Message::Assistant`]'s `raw`.
    pub raw: String,
}

/// A model service. Calls block; the app runs them off the UI thread.
pub trait Model: Send + Sync {
    /// Produce the next reply for a conversation and the advertised tools.
    fn reply(&self, messages: &[Message], tools: &[Tool]) -> Result<Reply, String>;

    /// Like [`Model::reply`], calling `text` with each piece of the answer
    /// as it arrives so it can be shown while the model writes. Services
    /// that can't stream send it in one piece.
    fn reply_streaming(
        &self,
        messages: &[Message],
        tools: &[Tool],
        text: &mut dyn FnMut(&str),
    ) -> Result<Reply, String> {
        let reply = self.reply(messages, tools)?;
        if !reply.text.is_empty() {
            text(&reply.text);
        }
        Ok(reply)
    }

    /// The models the service offers, to check the key and address work
    /// and to choose from. Services that can't list say so.
    fn models(&self) -> Result<Vec<String>, String> {
        Err("This service can't list its models.".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Stub(Result<Reply, String>);

    impl Model for Stub {
        fn reply(&self, _messages: &[Message], _tools: &[Tool]) -> Result<Reply, String> {
            self.0.clone()
        }
    }

    #[test]
    fn default_streaming_forwards_the_complete_text_once() {
        let model = Stub(Ok(Reply {
            text: "complete answer".into(),
            calls: Vec::new(),
            raw: String::new(),
        }));
        let mut pieces = Vec::new();

        let reply = model
            .reply_streaming(&[], &[], &mut |piece| pieces.push(piece.to_owned()))
            .unwrap();

        assert_eq!(reply.text, "complete answer");
        assert_eq!(pieces, ["complete answer"]);
    }

    #[test]
    fn default_streaming_does_not_emit_empty_text_for_tool_only_replies() {
        let call = ToolCall {
            id: "call-1".into(),
            name: "read_page".into(),
            arguments: "{}".into(),
        };
        let model = Stub(Ok(Reply {
            text: String::new(),
            calls: vec![call.clone()],
            raw: String::new(),
        }));
        let mut pieces = Vec::new();

        let reply = model
            .reply_streaming(&[], &[], &mut |piece| pieces.push(piece.to_owned()))
            .unwrap();

        assert!(pieces.is_empty());
        assert_eq!(reply.calls, [call]);
    }

    #[test]
    fn default_streaming_propagates_errors_without_emitting_text() {
        let model = Stub(Err("service unavailable".into()));
        let mut pieces = Vec::new();

        let error = model
            .reply_streaming(&[], &[], &mut |piece| pieces.push(piece.to_owned()))
            .unwrap_err();

        assert_eq!(error, "service unavailable");
        assert!(pieces.is_empty());
    }
}
