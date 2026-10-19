//! Anthropic's Messages API, for Claude models.

use std::io::{BufRead, BufReader};

use serde_json::{Value, json};

use crate::http::{self, MAX_REPLY_BYTES};
use crate::model::{Message, Model, Reply, ToolCall};
use crate::openai::endpoint;
use crate::tool::Tool;

/// The API version Ion speaks.
const VERSION: &str = "2023-06-01";

/// Room for the answer and the model's thinking. Streaming keeps a long
/// reply from timing out.
const MAX_TOKENS: u32 = 64_000;

/// Models that take the server-side fallback on a refusal.
const FALLBACK_MODELS: [&str; 4] = [
    "claude-fable-5-1",
    "claude-opus-5-5",
    "claude-opus-5",
    "claude-sonnet-5-5",
];
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

pub struct Anthropic {
    http: ureq::Agent,
    base_url: String,
    model: String,
    api_key: Option<String>,
}

impl Anthropic {
    /// `base_url` goes up to and including `/v1`.
    pub fn new(base_url: &str, model: &str, api_key: Option<String>) -> Anthropic {
        Anthropic {
            http: http::client(),
            base_url: base_url.trim_end_matches('/').to_owned(),
            model: model.to_owned(),
            api_key: api_key.filter(|k| !k.trim().is_empty()),
        }
    }

    fn headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = vec![("anthropic-version", VERSION.to_owned())];
        if let Some(key) = &self.api_key {
            headers.push(("x-api-key", key.clone()));
        }
        if takes_fallback(&self.model) {
            headers.push(("anthropic-beta", FALLBACK_BETA.to_owned()));
        }
        headers
    }
}

fn takes_fallback(model: &str) -> bool {
    FALLBACK_MODELS.contains(&model)
}

impl Model for Anthropic {
    fn reply(&self, messages: &[Message], tools: &[Tool]) -> Result<Reply, String> {
        self.reply_streaming(messages, tools, &mut |_| {})
    }

    fn reply_streaming(
        &self,
        messages: &[Message],
        tools: &[Tool],
        text: &mut dyn FnMut(&str),
    ) -> Result<Reply, String> {
        let url = endpoint(&self.base_url, self.api_key.is_some(), "messages")?;
        let body = request_body(&self.model, messages, tools);
        let mut response = http::post_json(&self.http, &url, &self.headers(), &body)?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            let error = response
                .body_mut()
                .with_config()
                .limit(MAX_REPLY_BYTES)
                .read_to_string()
                .unwrap_or_default();
            return Err(crate::openai::describe_error(status, &error));
        }
        let reader = response
            .body_mut()
            .with_config()
            .limit(MAX_REPLY_BYTES)
            .reader();
        let mut stream = Stream::default();
        for line in BufReader::new(reader).lines() {
            let line = line.map_err(|e| format!("The model service's reply was cut off: {e}"))?;
            if let Some(piece) = stream.line(&line)? {
                text(&piece);
            }
            if stream.done {
                break;
            }
        }
        stream.finish()
    }

    fn models(&self) -> Result<Vec<String>, String> {
        let url = endpoint(&self.base_url, self.api_key.is_some(), "models")?;
        let mut request = self.http.get(&url);
        for (name, value) in self.headers() {
            request = request.header(name, value);
        }
        let mut response = request
            .call()
            .map_err(|e| format!("Couldn't reach the model service: {e}"))?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_REPLY_BYTES)
            .read_to_string()
            .map_err(|e| format!("The model service's reply was cut off: {e}"))?;
        if !(200..300).contains(&status) {
            return Err(crate::openai::describe_error(status, &body));
        }
        crate::openai::parse_models(&body)
    }
}

/// The request for `/v1/messages`. System messages become the top-level
/// `system`; tool results go back in user turns, consecutive ones together.
pub fn request_body(model: &str, messages: &[Message], tools: &[Tool]) -> Value {
    let mut system = Vec::new();
    // (role, content blocks), neighbours with the same role merged.
    let mut turns: Vec<(&str, Vec<Value>)> = Vec::new();
    let mut push = |role: &'static str, blocks: Vec<Value>| match turns.last_mut() {
        Some((last, content)) if *last == role => content.extend(blocks),
        _ => turns.push((role, blocks)),
    };
    for message in messages {
        match message {
            Message::System(text) => system.push(text.as_str()),
            Message::User(text) => push("user", vec![json!({ "type": "text", "text": text })]),
            Message::Assistant { text, calls, raw } => {
                // Anthropic's own blocks, thinking included, go back as they
                // came; other services' turns are rebuilt.
                let blocks = serde_json::from_str::<Value>(raw)
                    .ok()
                    .and_then(|v| v.as_array().cloned())
                    .filter(|b| !b.is_empty())
                    .unwrap_or_else(|| {
                        let mut blocks = Vec::new();
                        if !text.trim().is_empty() {
                            blocks.push(json!({ "type": "text", "text": text }));
                        }
                        for call in calls {
                            let input = serde_json::from_str::<Value>(&call.arguments)
                                .ok()
                                .filter(Value::is_object)
                                .unwrap_or_else(|| json!({}));
                            blocks.push(json!({
                                "type": "tool_use",
                                "id": call.id,
                                "name": call.name,
                                "input": input,
                            }));
                        }
                        blocks
                    });
                if !blocks.is_empty() {
                    push("assistant", blocks);
                }
            }
            Message::Tool { call_id, content } => push(
                "user",
                vec![json!({ "type": "tool_result", "tool_use_id": call_id, "content": content })],
            ),
        }
    }
    let messages: Vec<Value> = turns
        .into_iter()
        .map(|(role, content)| json!({ "role": role, "content": content }))
        .collect();
    let mut body = json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "stream": true,
        "messages": messages,
    });
    if !system.is_empty() {
        body["system"] = Value::from(system.join("\n\n"));
    }
    if takes_fallback(model) {
        body["fallbacks"] = Value::from("default");
    }
    if !tools.is_empty() {
        let tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name(),
                    "description": t.description(),
                    "input_schema": t.parameters(),
                    "eager_input_streaming": true,
                })
            })
            .collect();
        body["tools"] = Value::from(tools);
    }
    body
}

/// A streamed `/v1/messages` reply, put together event by event. The
/// content blocks are kept whole so they can be sent back unchanged.
#[derive(Debug, Default)]
pub struct Stream {
    blocks: Vec<Value>,
    /// Tool input JSON as it arrives, per block.
    inputs: Vec<String>,
    stop_reason: String,
    /// The service said `message_stop`.
    pub done: bool,
}

impl Stream {
    /// Read one line of the event stream. Returns the new answer text it
    /// carried, if any.
    pub fn line(&mut self, line: &str) -> Result<Option<String>, String> {
        let Some(data) = line.strip_prefix("data:") else {
            return Ok(None);
        };
        let event: Value = serde_json::from_str(data.trim())
            .map_err(|_| "The model service sent something that isn't a chat reply.".to_owned())?;
        let index = || {
            event["index"]
                .as_u64()
                .and_then(|i| usize::try_from(i).ok())
                .filter(|i| *i <= 256)
                .ok_or_else(|| "The model service sent a broken reply.".to_owned())
        };
        match event["type"].as_str().unwrap_or_default() {
            "content_block_start" => {
                let index = index()?;
                while self.blocks.len() <= index {
                    self.blocks.push(Value::Null);
                    self.inputs.push(String::new());
                }
                self.blocks[index] = event["content_block"].clone();
            }
            "content_block_delta" => {
                let index = index()?;
                let delta = &event["delta"];
                let Some(block) = self.blocks.get_mut(index) else {
                    return Ok(None);
                };
                let mut append = |key: &str, piece: &str| {
                    let mut text = block[key].as_str().unwrap_or_default().to_owned();
                    text.push_str(piece);
                    block[key] = Value::from(text);
                };
                match delta["type"].as_str().unwrap_or_default() {
                    "text_delta" => {
                        let piece = delta["text"].as_str().unwrap_or_default();
                        append("text", piece);
                        if !piece.is_empty() {
                            return Ok(Some(piece.to_owned()));
                        }
                    }
                    "thinking_delta" => {
                        append("thinking", delta["thinking"].as_str().unwrap_or_default());
                    }
                    "signature_delta" => {
                        block["signature"] = delta["signature"].clone();
                    }
                    "input_json_delta" => {
                        self.inputs[index]
                            .push_str(delta["partial_json"].as_str().unwrap_or_default());
                    }
                    _ => {}
                }
            }
            "message_delta" => {
                if let Some(reason) = event["delta"]["stop_reason"].as_str() {
                    reason.clone_into(&mut self.stop_reason);
                }
            }
            "message_stop" => self.done = true,
            "error" => {
                let message = event["error"]["message"]
                    .as_str()
                    .unwrap_or("something went wrong");
                return Err(format!("The model service had a problem: {message}"));
            }
            _ => {}
        }
        Ok(None)
    }

    pub fn finish(mut self) -> Result<Reply, String> {
        if self.stop_reason == "refusal" {
            return Err("The model declined to help with this.".to_owned());
        }
        let mut text = String::new();
        let mut calls = Vec::new();
        for (block, input) in self.blocks.iter_mut().zip(&self.inputs) {
            match block["type"].as_str().unwrap_or_default() {
                "text" => text.push_str(block["text"].as_str().unwrap_or_default()),
                "tool_use" => {
                    let arguments = if input.trim().is_empty() {
                        "{}".to_owned()
                    } else {
                        input.clone()
                    };
                    // The block goes back with its input as an object; a
                    // cut-off input reads as empty and the tool says what's
                    // missing.
                    block["input"] = serde_json::from_str::<Value>(&arguments)
                        .ok()
                        .filter(Value::is_object)
                        .unwrap_or_else(|| json!({}));
                    calls.push(ToolCall {
                        id: block["id"].as_str().unwrap_or_default().to_owned(),
                        name: block["name"].as_str().unwrap_or_default().to_owned(),
                        arguments,
                    });
                }
                _ => {}
            }
        }
        if self.stop_reason == "max_tokens" && calls.is_empty() && text.trim().is_empty() {
            return Err("The model ran out of room before answering.".to_owned());
        }
        self.blocks.retain(|b| !b.is_null());
        Ok(Reply {
            text,
            calls,
            raw: Value::from(self.blocks).to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(lines: &[&str]) -> (Reply, String) {
        let mut stream = Stream::default();
        let mut shown = String::new();
        for line in lines {
            if let Some(piece) = stream.line(line).unwrap() {
                shown.push_str(&piece);
            }
        }
        assert!(stream.done);
        (stream.finish().unwrap(), shown)
    }

    #[test]
    fn streams_are_put_together_with_thinking_kept() {
        let (reply, shown) = stream(&[
            "event: message_start",
            r#"data: {"type":"message_start","message":{"id":"m1","role":"assistant","content":[]}}"#,
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}"#,
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Read it first."}}"#,
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig"}}"#,
            r#"data: {"type":"content_block_stop","index":0}"#,
            r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Reading "}}"#,
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"the page."}}"#,
            r#"data: {"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"toolu_1","name":"read_page","input":{}}}"#,
            r#"data: {"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"ta"}}"#,
            r#"data: {"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"b\": 3}"}}"#,
            r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use"}}"#,
            r#"data: {"type":"message_stop"}"#,
        ]);
        assert_eq!(shown, "Reading the page.");
        assert_eq!(reply.text, "Reading the page.");
        assert_eq!(
            reply.calls,
            [ToolCall {
                id: "toolu_1".into(),
                name: "read_page".into(),
                arguments: r#"{"tab": 3}"#.into(),
            }]
        );
        let raw: Value = serde_json::from_str(&reply.raw).unwrap();
        assert_eq!(raw[0]["type"], "thinking");
        assert_eq!(raw[0]["thinking"], "Read it first.");
        assert_eq!(raw[0]["signature"], "sig");
        assert_eq!(raw[2]["input"]["tab"], 3);
    }

    #[test]
    fn refusals_and_errors_are_sentences() {
        let mut refused = Stream::default();
        refused
            .line(r#"data: {"type":"message_delta","delta":{"stop_reason":"refusal"}}"#)
            .unwrap();
        assert!(refused.finish().is_err());

        let mut failed = Stream::default();
        let error = failed
            .line(r#"data: {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#)
            .unwrap_err();
        assert!(error.contains("Overloaded"));
        assert!(failed.line("data: <html>").is_err());
        assert!(
            failed
                .line(r#"data: {"type":"content_block_start","index":99999,"content_block":{}}"#)
                .is_err()
        );
    }

    #[test]
    fn builds_a_messages_request() {
        let messages = [
            Message::System("sys".into()),
            Message::User("hi".into()),
            Message::Assistant {
                text: String::new(),
                calls: vec![
                    ToolCall {
                        id: "c1".into(),
                        name: "read_page".into(),
                        arguments: "{}".into(),
                    },
                    ToolCall {
                        id: "c2".into(),
                        name: "list_tabs".into(),
                        arguments: "not json".into(),
                    },
                ],
                raw: String::new(),
            },
            Message::Tool {
                call_id: "c1".into(),
                content: "<page>…</page>".into(),
            },
            Message::Tool {
                call_id: "c2".into(),
                content: "<tabs>…</tabs>".into(),
            },
            Message::User("and then?".into()),
        ];
        let body = request_body("claude-opus-5-5", &messages, &Tool::ALL);
        assert_eq!(body["system"], "sys");
        assert_eq!(body["fallbacks"], "default");
        assert_eq!(body["stream"], true);
        let turns = body["messages"].as_array().unwrap();
        assert_eq!(
            turns.len(),
            3,
            "tool results and the next question share a turn"
        );
        assert_eq!(turns[1]["role"], "assistant");
        assert_eq!(turns[1]["content"][0]["type"], "tool_use");
        assert_eq!(turns[1]["content"][1]["input"], json!({}));
        assert_eq!(turns[2]["role"], "user");
        assert_eq!(turns[2]["content"][0]["tool_use_id"], "c1");
        assert_eq!(turns[2]["content"][1]["tool_use_id"], "c2");
        assert_eq!(turns[2]["content"][2]["text"], "and then?");
        assert_eq!(body["tools"][0]["name"], "read_page");
        assert!(body["tools"][0]["input_schema"].is_object());

        let plain = request_body("claude-haiku-5-5", &messages[..2], &[]);
        assert!(plain.get("fallbacks").is_none());
        assert!(plain.get("tools").is_none());
    }

    #[test]
    fn its_own_blocks_go_back_unchanged() {
        let raw = r#"[{"type":"thinking","thinking":"","signature":"abc"},{"type":"tool_use","id":"t","name":"list_tabs","input":{}}]"#;
        let messages = [
            Message::User("q".into()),
            Message::Assistant {
                text: String::new(),
                calls: Vec::new(),
                raw: raw.into(),
            },
        ];
        let body = request_body("claude-opus-5-5", &messages, &[]);
        assert_eq!(body["messages"][1]["content"][0]["signature"], "abc");
    }
}
