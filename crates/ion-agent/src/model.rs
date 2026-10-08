//! The conversation with a model, independent of any one service's format.

use crate::tool::Tool;

/// A call the model asked for: `name` is a [`Tool`] name and `arguments`
/// the JSON text it sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    System(String),
    User(String),
    Assistant {
        text: String,
        calls: Vec<ToolCall>,
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
    pub text: String,
    pub calls: Vec<ToolCall>,
}

/// A model service. Calls block; the app runs them off the UI thread.
pub trait Model: Send + Sync {
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
}
