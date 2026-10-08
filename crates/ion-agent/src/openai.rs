//! Any OpenAI-compatible chat completions API: OpenAI itself, OpenRouter,
//! or a local server such as Ollama or llama.cpp.

use std::io::{BufRead, BufReader};
use std::time::Duration;

use serde_json::{Value, json};

use crate::model::{Message, Model, Reply, ToolCall};
use crate::tool::Tool;

/// Answers bigger than this are not a chat reply.
const MAX_REPLY_BYTES: u64 = 4 * 1024 * 1024;

pub struct OpenAi {
    http: ureq::Agent,
    base_url: String,
    model: String,
    api_key: Option<String>,
}

impl OpenAi {
    /// `base_url` goes up to and including `/v1`. Honours the usual
    /// `HTTPS_PROXY` variables.
    pub fn new(base_url: &str, model: &str, api_key: Option<String>) -> OpenAi {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(120)))
            // Error bodies say what went wrong; read them instead of
            // getting a bare status.
            .http_status_as_error(false)
            .user_agent(concat!("Ion/", env!("CARGO_PKG_VERSION")))
            .build();
        OpenAi {
            http: config.into(),
            base_url: base_url.trim_end_matches('/').to_owned(),
            model: model.to_owned(),
            api_key: api_key.filter(|k| !k.trim().is_empty()),
        }
    }
}

impl OpenAi {
    fn post(&self, body: &Value) -> Result<ureq::http::Response<ureq::Body>, String> {
        let mut request = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .header("Content-Type", "application/json");
        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {key}"));
        }
        request
            .send(body.to_string())
            .map_err(|e| format!("Couldn't reach the model service: {e}"))
    }
}

impl Model for OpenAi {
    fn reply_streaming(
        &self,
        messages: &[Message],
        tools: &[Tool],
        text: &mut dyn FnMut(&str),
    ) -> Result<Reply, String> {
        let mut body = request_body(&self.model, messages, tools);
        body["stream"] = Value::Bool(true);
        let mut response = self.post(&body)?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            let error = response
                .body_mut()
                .with_config()
                .limit(MAX_REPLY_BYTES)
                .read_to_string()
                .unwrap_or_default();
            return Err(describe_error(status, &error));
        }
        // A server that ignores `stream` answers with plain JSON.
        let is_events = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("text/event-stream"));
        let reader = response
            .body_mut()
            .with_config()
            .limit(MAX_REPLY_BYTES)
            .reader();
        if !is_events {
            let mut all = String::new();
            BufReader::new(reader)
                .lines()
                .try_for_each(|line| {
                    all.push_str(&line?);
                    all.push('\n');
                    Ok::<_, std::io::Error>(())
                })
                .map_err(|e| format!("The model service's reply was cut off: {e}"))?;
            let reply = parse_reply(&all)?;
            if !reply.text.is_empty() {
                text(&reply.text);
            }
            return Ok(reply);
        }
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
        Ok(stream.finish())
    }

    fn reply(&self, messages: &[Message], tools: &[Tool]) -> Result<Reply, String> {
        let mut response = self.post(&request_body(&self.model, messages, tools))?;
        let status = response.status().as_u16();
        let text = response
            .body_mut()
            .with_config()
            .limit(MAX_REPLY_BYTES)
            .read_to_string()
            .map_err(|e| format!("The model service's reply was cut off: {e}"))?;
        if !(200..300).contains(&status) {
            return Err(describe_error(status, &text));
        }
        parse_reply(&text)
    }
}

/// A streamed `/chat/completions` reply, put together line by line from
/// its server-sent events.
#[derive(Debug, Default)]
pub struct Stream {
    text: String,
    calls: Vec<ToolCall>,
    /// The service said `[DONE]`.
    pub done: bool,
}

impl Stream {
    /// Read one line of the event stream. Returns the new answer text it
    /// carried, if any.
    pub fn line(&mut self, line: &str) -> Result<Option<String>, String> {
        let Some(data) = line.strip_prefix("data:") else {
            return Ok(None);
        };
        let data = data.trim();
        if data == "[DONE]" {
            self.done = true;
            return Ok(None);
        }
        let value: Value = serde_json::from_str(data)
            .map_err(|_| "The model service sent something that isn't a chat reply.".to_owned())?;
        if let Some(message) = value["error"]["message"].as_str() {
            return Err(format!("The model service had a problem: {message}"));
        }
        let delta = &value["choices"][0]["delta"];
        for call in delta["tool_calls"].as_array().into_iter().flatten() {
            let index = call["index"]
                .as_u64()
                .and_then(|i| usize::try_from(i).ok())
                .unwrap_or(self.calls.len());
            // A service sending absurd indexes isn't a chat reply.
            if index > 64 {
                return Err("The model service asked for too many tools at once.".to_owned());
            }
            while self.calls.len() <= index {
                let id = format!("call_{}", self.calls.len());
                self.calls.push(ToolCall {
                    id,
                    name: String::new(),
                    arguments: String::new(),
                });
            }
            let entry = &mut self.calls[index];
            if let Some(id) = call["id"].as_str() {
                entry.id = id.to_owned();
            }
            if let Some(name) = call["function"]["name"].as_str() {
                entry.name.push_str(name);
            }
            if let Some(arguments) = call["function"]["arguments"].as_str() {
                entry.arguments.push_str(arguments);
            }
        }
        match delta["content"].as_str() {
            Some(piece) if !piece.is_empty() => {
                self.text.push_str(piece);
                Ok(Some(piece.to_owned()))
            }
            _ => Ok(None),
        }
    }

    pub fn finish(self) -> Reply {
        let calls = self
            .calls
            .into_iter()
            .map(|mut c| {
                if c.arguments.trim().is_empty() {
                    c.arguments = "{}".to_owned();
                }
                c
            })
            .collect();
        Reply {
            text: self.text,
            calls,
        }
    }
}

/// The request for `/chat/completions`.
pub fn request_body(model: &str, messages: &[Message], tools: &[Tool]) -> Value {
    let messages: Vec<Value> = messages.iter().map(message_json).collect();
    let mut body = json!({ "model": model, "messages": messages });
    if !tools.is_empty() {
        let tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name(),
                        "description": t.description(),
                        "parameters": t.parameters(),
                    }
                })
            })
            .collect();
        body["tools"] = Value::from(tools);
    }
    body
}

fn message_json(message: &Message) -> Value {
    match message {
        Message::System(text) => json!({ "role": "system", "content": text }),
        Message::User(text) => json!({ "role": "user", "content": text }),
        Message::Assistant { text, calls } => {
            let mut m = json!({ "role": "assistant", "content": text });
            if !calls.is_empty() {
                let calls: Vec<Value> = calls
                    .iter()
                    .map(|c| {
                        json!({
                            "id": c.id,
                            "type": "function",
                            "function": { "name": c.name, "arguments": c.arguments },
                        })
                    })
                    .collect();
                m["tool_calls"] = Value::from(calls);
            }
            m
        }
        Message::Tool { call_id, content } => {
            json!({ "role": "tool", "tool_call_id": call_id, "content": content })
        }
    }
}

/// The first choice of a `/chat/completions` response.
pub fn parse_reply(text: &str) -> Result<Reply, String> {
    let value: Value = serde_json::from_str(text)
        .map_err(|_| "The model service sent something that isn't a chat reply.".to_owned())?;
    let message = &value["choices"][0]["message"];
    if message.is_null() {
        return Err("The model service sent a reply with no message.".to_owned());
    }
    let text = message["content"].as_str().unwrap_or_default().to_owned();
    let calls = message["tool_calls"]
        .as_array()
        .map(|calls| {
            calls
                .iter()
                .enumerate()
                .map(|(i, c)| ToolCall {
                    id: c["id"]
                        .as_str()
                        .map_or_else(|| format!("call_{i}"), str::to_owned),
                    name: c["function"]["name"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                    arguments: c["function"]["arguments"]
                        .as_str()
                        .unwrap_or("{}")
                        .to_owned(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Reply { text, calls })
}

/// A sentence for a failed request, using the service's own message when it
/// sent one.
pub fn describe_error(status: u16, body: &str) -> String {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_owned))
        .filter(|m| !m.trim().is_empty());
    let what = match status {
        401 | 403 => "The model service refused the API key",
        404 => "The model service doesn't know that model or address",
        429 => "The model service is rate limiting or out of credit",
        500..=599 => "The model service had a problem",
        _ => "The model service refused the request",
    };
    match detail {
        Some(detail) => format!("{what} ({status}): {}", detail.trim()),
        None => format!("{what} ({status})."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_are_put_together() {
        let mut stream = Stream::default();
        let lines = [
            ": keep-alive",
            r#"data: {"choices":[{"delta":{"role":"assistant","content":""}}]}"#,
            r#"data: {"choices":[{"delta":{"content":"Qt 6 "}}]}"#,
            "",
            r#"data: {"choices":[{"delta":{"content":"runs natively."}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c1","function":{"name":"read_","arguments":""}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"name":"page","arguments":"{\"ta"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"b\": 3}"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":1,"id":"c2","function":{"name":"list_tabs"}}]}}]}"#,
            "data: [DONE]",
        ];
        let mut shown = String::new();
        for line in lines {
            if let Some(piece) = stream.line(line).unwrap() {
                shown.push_str(&piece);
            }
        }
        assert!(stream.done);
        assert_eq!(shown, "Qt 6 runs natively.");
        let reply = stream.finish();
        assert_eq!(reply.text, "Qt 6 runs natively.");
        assert_eq!(
            reply.calls,
            [
                ToolCall {
                    id: "c1".into(),
                    name: "read_page".into(),
                    arguments: r#"{"tab": 3}"#.into()
                },
                ToolCall {
                    id: "c2".into(),
                    name: "list_tabs".into(),
                    arguments: "{}".into()
                },
            ]
        );
    }

    #[test]
    fn stream_errors_are_sentences() {
        let mut stream = Stream::default();
        let error = stream
            .line(r#"data: {"error":{"message":"overloaded"}}"#)
            .unwrap_err();
        assert!(error.contains("overloaded"));
        assert!(stream.line("data: <html>").is_err());
        assert!(
            stream
                .line(r#"data: {"choices":[{"delta":{"tool_calls":[{"index":9999}]}}]}"#)
                .is_err()
        );
    }

    #[test]
    fn builds_a_chat_request_with_tools() {
        let messages = [
            Message::System("sys".into()),
            Message::User("hi".into()),
            Message::Assistant {
                text: String::new(),
                calls: vec![ToolCall {
                    id: "c1".into(),
                    name: "read_page".into(),
                    arguments: "{}".into(),
                }],
            },
            Message::Tool {
                call_id: "c1".into(),
                content: "<page>…</page>".into(),
            },
        ];
        let body = request_body("gpt-x", &messages, &Tool::ALL);
        assert_eq!(body["model"], "gpt-x");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(
            body["messages"][2]["tool_calls"][0]["function"]["name"],
            "read_page"
        );
        assert_eq!(body["messages"][3]["role"], "tool");
        assert_eq!(body["messages"][3]["tool_call_id"], "c1");
        assert_eq!(body["tools"][1]["function"]["name"], "list_tabs");
        assert!(request_body("m", &messages, &[]).get("tools").is_none());
    }

    #[test]
    fn parses_answers_and_tool_calls() {
        let reply = parse_reply(
            r#"{"choices":[{"message":{"role":"assistant","content":null,
                "tool_calls":[{"id":"call_9","type":"function",
                "function":{"name":"read_page","arguments":"{}"}}]}}]}"#,
        )
        .unwrap();
        assert_eq!(reply.text, "");
        assert_eq!(reply.calls[0].id, "call_9");
        assert_eq!(reply.calls[0].name, "read_page");

        let reply =
            parse_reply(r#"{"choices":[{"message":{"role":"assistant","content":"Hi."}}]}"#)
                .unwrap();
        assert_eq!(reply.text, "Hi.");
        assert!(reply.calls.is_empty());

        assert!(parse_reply("<html>").is_err());
        assert!(parse_reply(r#"{"choices":[]}"#).is_err());
    }

    #[test]
    fn errors_say_what_to_fix() {
        assert_eq!(
            describe_error(401, r#"{"error":{"message":"Incorrect API key provided"}}"#),
            "The model service refused the API key (401): Incorrect API key provided"
        );
        assert_eq!(
            describe_error(502, "bad gateway"),
            "The model service had a problem (502)."
        );
    }
}
