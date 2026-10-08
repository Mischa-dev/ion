//! Ion Agent's runtime, without Qt.
//!
//! - [`invoke::parse`] reads `@ion …` or `!ai …` from the address bar.
//! - [`task::Task`] is one request to an agent, as a state machine the app
//!   drives: it says what to do next (ask the model, run a browser tool) and
//!   is told how that went. Nothing here blocks or spawns threads.
//! - [`tool::Tool`] lists the browser tools agents may call, each mapped to
//!   the [`ion_safety::Action`] the safety model decides on.
//! - [`openai::OpenAi`] talks to any OpenAI-compatible chat completions API.
//!
//! Every tool call goes through `ion-safety` first ([`tool::plan`]);
//! page content reaches the model only inside marked blocks the system
//! prompt tells it to treat as data (docs/SAFETY.md §5).

pub mod invoke;
pub mod markdown;
pub mod model;
pub mod openai;
pub mod task;
pub mod tool;

pub use invoke::Invocation;
pub use model::{Message, Model, Reply, ToolCall};
pub use task::{Next, Status, Step, StepStatus, Task, ToolOutcome};
pub use tool::Tool;
