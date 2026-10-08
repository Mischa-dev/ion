//! `Agent` QML singleton: runs Ion Agent tasks (`ion_agent::Task`).
//!
//! The address bar starts a task with `start`. Model calls run on a worker
//! thread; browser tools run in QML, because they read web views:
//!
//! 1. `toolRequested(task, call, tool, tab)`: QML looks up the tab's address
//!    and calls `authorize(task, call, url)`, which checks the call and asks
//!    the safety model.
//! 2. "allow": QML reads the call with `callJson`, runs it and reports with
//!    `toolResult`. "ask": the task's JSON carries a prompt; the person's
//!    answer goes to `answer`, and `toolApproved(task, call)` follows if they
//!    allowed it. "deny": the task carries on without it. "paused": the
//!    person has the tab (`takeOver`); `toolRequested` comes again after
//!    `handBack`.
//!
//! Every change emits `taskChanged(task)`; `taskJson` describes a task.
//! Nothing runs until `[ai] enable = true`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, revision, READ, NOTIFY)]
        #[namespace = "ion"]
        type Agent = super::AgentRust;

        /// Address-bar rows for `input` when it is addressed to an agent
        /// (`@ion …`, `!ai …`), as `PaletteSearch` rows in JSON; `"[]"`
        /// otherwise or while agents are off.
        #[qinvokable]
        fn suggest(self: &Agent, input: &QString) -> QString;

        /// Start the task `input` asks for (`@ion …` or `!ai …`) on the tab
        /// `tab`, now showing `url` titled `title`. Returns the task id, or
        /// -1 when `input` isn't for an agent or agents are off.
        #[qinvokable]
        fn start(
            self: Pin<&mut Agent>,
            input: &QString,
            tab: i32,
            url: &QString,
            title: &QString,
        ) -> i32;

        /// The task as JSON: `{id, agent, agentName, tab, question, status,
        /// steps: [{text, status}], answer, error, prompt, acting,
        /// takenOver}`, where `prompt` is `{text, detail, choices: [{id,
        /// label}]}` while waiting for the person, `acting` says the agent
        /// has been changing the page, and `takenOver` that the person took
        /// the tab back. Empty for unknown ids.
        #[qinvokable]
        #[cxx_name = "taskJson"]
        fn task_json(self: &Agent, task: i32) -> QString;

        /// The newest task started on `tab`, or -1.
        #[qinvokable]
        #[cxx_name = "latestTask"]
        fn latest_task(self: &Agent, tab: i32) -> i32;

        /// Ask the safety model about the tool call `call` of `task`, whose
        /// tab now shows `url`: "allow", "ask", "deny", or "paused" while the
        /// person has taken the tab back (`toolRequested` comes again once
        /// they hand it back).
        #[qinvokable]
        fn authorize(self: Pin<&mut Agent>, task: i32, call: &QString, url: &QString) -> QString;

        /// What an allowed call does, for the code that runs it: `{tool,
        /// element, text, url, allowSubmit}`.
        #[qinvokable]
        #[cxx_name = "callJson"]
        fn call_json(self: &Agent, task: i32, call: &QString) -> QString;

        /// The person takes the task's tab back: the agent waits before its
        /// next step on it until `handBack`.
        #[qinvokable]
        #[cxx_name = "takeOver"]
        fn take_over(self: Pin<&mut Agent>, task: i32);

        #[qinvokable]
        #[cxx_name = "handBack"]
        fn hand_back(self: Pin<&mut Agent>, task: i32);

        /// Whether an agent is at work changing the page in `tab`, for the
        /// ring on its tab icon.
        #[qinvokable]
        #[cxx_name = "actingOn"]
        fn acting_on(self: &Agent, tab: i32) -> bool;

        /// The person answered the task's prompt with a choice id.
        #[qinvokable]
        fn answer(self: Pin<&mut Agent>, task: i32, choice: &QString);

        /// A tool finished. `payload` is JSON: `{url, title, text,
        /// elements}` for read_page, `{tabs: [{title, url}]}` for
        /// list_tabs, `{url, title}` for go_to, `{}` for click and fill, or
        /// `{error}` when `ok` is false.
        #[qinvokable]
        #[cxx_name = "toolResult"]
        fn tool_result(
            self: Pin<&mut Agent>,
            task: i32,
            call: &QString,
            ok: bool,
            payload: &QString,
        );

        #[qinvokable]
        fn stop(self: Pin<&mut Agent>, task: i32);

        /// Stop every running task (the global stop).
        #[qinvokable]
        #[cxx_name = "stopAll"]
        fn stop_all(self: Pin<&mut Agent>);

        /// Forget a finished task.
        #[qinvokable]
        fn dismiss(self: Pin<&mut Agent>, task: i32);

        /// An answer's Markdown as the small HTML subset `Text` renders:
        /// paragraphs, lists, `code` and **bold**.
        #[qinvokable]
        #[cxx_name = "answerHtml"]
        fn answer_html(self: &Agent, markdown: &QString) -> QString;

        #[qsignal]
        #[cxx_name = "taskChanged"]
        fn task_changed(self: Pin<&mut Agent>, task: i32);

        #[qsignal]
        #[cxx_name = "toolRequested"]
        fn tool_requested(
            self: Pin<&mut Agent>,
            task: i32,
            call: &QString,
            tool: &QString,
            tab: i32,
        );

        #[qsignal]
        #[cxx_name = "toolApproved"]
        fn tool_approved(self: Pin<&mut Agent>, task: i32, call: &QString);
    }

    impl cxx_qt::Threading for Agent {}
}

use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::Arc;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use ion_agent::model::Model;
use ion_agent::openai::OpenAi;
use ion_agent::task::Context;
use ion_agent::tool::{Element, Plan, PlanError, Use, page_result, plan, tabs_result};
use ion_agent::{Next, Reply, Task, Tool, ToolCall, ToolOutcome, invoke};
use ion_safety::{Choice, Prompt, Verdict};
use serde_json::{Value, json};

/// A tool call waiting for the person's answer.
struct Pending {
    call: String,
    prompt: Prompt,
}

struct Running {
    task: Task,
    /// The call being run and its tool, between `toolRequested` and
    /// `toolResult`.
    call: Option<(ToolCall, Tool)>,
    /// The checked call, once `authorize` has read it.
    plan: Option<Plan>,
    pending: Option<Pending>,
    /// The call waiting for the person to hand the tab back.
    paused: Option<String>,
    /// What the last read of the page numbered.
    elements: Vec<Element>,
    /// The agent has been changing the page.
    acted: bool,
}

#[derive(Default)]
pub struct AgentRust {
    revision: i32,
    tasks: BTreeMap<i32, Running>,
    next_id: i32,
}

fn enabled() -> bool {
    ion_config::global().config().ai.enable
}

/// The model the config names, with its key from the environment.
fn configured_model() -> Result<Arc<dyn Model>, String> {
    let ai = ion_config::global().config().ai.clone();
    if ai.provider != "openai" {
        return Err(format!(
            "Ion Agent can't use the {:?} provider yet. Set ai.provider to \"openai\".",
            ai.provider
        ));
    }
    let key = if ai.api_key_env.trim().is_empty() {
        None
    } else {
        match std::env::var(ai.api_key_env.trim()) {
            Ok(key) if !key.trim().is_empty() => Some(key),
            _ => {
                return Err(format!(
                    "No API key: set {} in Ion's environment, or set ai.apiKeyEnv to \"\" for a local model.",
                    ai.api_key_env.trim()
                ));
            }
        }
    };
    Ok(Arc::new(OpenAi::new(&ai.base_url, &ai.model, key)))
}

fn host_of(url: &str) -> Option<String> {
    ion_safety::Origin::parse(url).and_then(|o| o.host().map(str::to_owned))
}

fn is_taken_over(tab: u64) -> bool {
    ion_safety::global().policy().is_taken_over(tab)
}

impl qobject::Agent {
    fn bump(mut self: Pin<&mut Self>, task: i32) {
        let revision = self.revision.wrapping_add(1);
        self.as_mut().rust_mut().revision = revision;
        self.as_mut().revision_changed();
        self.task_changed(task);
    }

    fn suggest(&self, input: &QString) -> QString {
        let input = input.to_string();
        if !enabled() {
            return QString::from("[]");
        }
        let rows = if let Some(ask) = invoke::parse(&input) {
            let name = ion_safety::global().policy().display_name(&ask.agent);
            vec![json!({
                "kind": "agent",
                "title": format!("Ask {name}"),
                "subtitle": ask.question,
                "hint": "Enter",
                "action": "agent",
                "value": input,
            })]
        } else if let Some(prefix) = invoke::agent_prefix(&input) {
            let id = invoke::DEFAULT_AGENT;
            if id.starts_with(prefix) {
                vec![json!({
                    "kind": "agent",
                    "title": ion_safety::global().policy().display_name(id),
                    "subtitle": "Ask about this page or get something done on it",
                    "hint": "Tab",
                    "action": "complete",
                    "value": format!("@{id} "),
                })]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        QString::from(Value::from(rows).to_string().as_str())
    }

    fn start(
        mut self: Pin<&mut Self>,
        input: &QString,
        tab: i32,
        url: &QString,
        title: &QString,
    ) -> i32 {
        let Some(ask) = invoke::parse(&input.to_string()) else {
            return -1;
        };
        let Ok(tab_id) = u64::try_from(tab) else {
            return -1;
        };
        if !enabled() {
            return -1;
        }
        let id = self.next_id + 1;
        self.as_mut().rust_mut().next_id = id;
        let context = Context {
            title: title.to_string(),
            url: url.to_string(),
        };
        let mut task = Task::new(id as u64, &ask.agent, tab_id, &ask.question, &context);
        if ask.agent != invoke::DEFAULT_AGENT {
            task.model_failed(format!(
                "Only Ion Agent can be asked from the address bar so far, not @{}.",
                ask.agent
            ));
        }
        // Asking from a tab hands it to the agent, even if the person took
        // it back from an earlier task.
        ion_safety::global().hand_back(tab_id);
        self.as_mut().rust_mut().tasks.insert(
            id,
            Running {
                task,
                call: None,
                plan: None,
                pending: None,
                paused: None,
                elements: Vec::new(),
                acted: false,
            },
        );
        self.as_mut().pump(id);
        id
    }

    /// Move `id` along until it waits on something.
    fn pump(mut self: Pin<&mut Self>, id: i32) {
        loop {
            let next = match self.as_mut().rust_mut().tasks.get_mut(&id) {
                Some(running) => running.task.advance(),
                None => return,
            };
            match next {
                Next::Model(messages) => {
                    match configured_model() {
                        Ok(model) => {
                            let thread = self.qt_thread();
                            std::thread::spawn(move || {
                                let reply = model.reply(&messages, &Tool::ALL);
                                let _ = thread.queue(move |obj| obj.model_done(id, reply));
                            });
                        }
                        Err(error) => {
                            if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&id) {
                                running.task.model_failed(error);
                            }
                            continue;
                        }
                    }
                    break;
                }
                Next::Tool(call, tool) => {
                    let tab = self.tasks.get(&id).map_or(-1, |r| r.task.tab as i32);
                    let call_id = QString::from(call.id.as_str());
                    if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&id) {
                        running.call = Some((call, tool));
                        running.plan = None;
                    }
                    self.as_mut().bump(id);
                    self.as_mut()
                        .tool_requested(id, &call_id, &QString::from(tool.name()), tab);
                    return;
                }
                Next::Idle | Next::Finished => break,
            }
        }
        self.bump(id);
    }

    fn model_done(mut self: Pin<&mut Self>, id: i32, reply: Result<Reply, String>) {
        if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&id) {
            match reply {
                Ok(reply) => running.task.model_replied(reply),
                Err(error) => running.task.model_failed(error),
            }
            self.pump(id);
        }
    }

    fn task_json(&self, task: i32) -> QString {
        let Some(running) = self.tasks.get(&task) else {
            return QString::default();
        };
        let over = running.task.status().is_over();
        let mut value = running.task.to_json();
        value["agentName"] = Value::from(
            ion_safety::global()
                .policy()
                .display_name(&running.task.agent),
        );
        value["prompt"] = match &running.pending {
            Some(pending) => {
                let choices: Vec<Value> = pending
                    .prompt
                    .choices
                    .iter()
                    .map(|(choice, label)| json!({ "id": choice.id(), "label": label }))
                    .collect();
                let detail = running.plan.as_ref().map_or("", |p| p.detail.as_str());
                json!({ "text": pending.prompt.text, "detail": detail, "choices": choices })
            }
            None => Value::Null,
        };
        value["acting"] = Value::from(running.acted && !over);
        value["takenOver"] = Value::from(!over && is_taken_over(running.task.tab));
        QString::from(value.to_string().as_str())
    }

    fn latest_task(&self, tab: i32) -> i32 {
        self.tasks
            .iter()
            .rev()
            .find(|(_, r)| r.task.tab as i32 == tab)
            .map_or(-1, |(id, _)| *id)
    }

    fn acting_on(&self, tab: i32) -> bool {
        self.tasks
            .values()
            .any(|r| r.task.tab as i32 == tab && r.acted && !r.task.status().is_over())
            && !u64::try_from(tab).is_ok_and(is_taken_over)
    }

    /// The running call `call` of `task`, if that is what's running.
    fn current_call(&self, task: i32, call: &str) -> Option<(ToolCall, Tool, u64, String)> {
        let running = self.tasks.get(&task)?;
        let (c, tool) = running.call.as_ref()?;
        (c.id == call).then(|| {
            (
                c.clone(),
                *tool,
                running.task.tab,
                running.task.agent.clone(),
            )
        })
    }

    fn authorize(mut self: Pin<&mut Self>, task: i32, call: &QString, url: &QString) -> QString {
        let call = call.to_string();
        let url = url.to_string();
        let Some((c, tool, tab, agent)) = self.current_call(task, &call) else {
            return QString::from("deny");
        };
        let elements = self
            .tasks
            .get(&task)
            .map(|r| r.elements.clone())
            .unwrap_or_default();
        let planned = Use::parse(tool, &c.arguments)
            .map_err(PlanError::Mistake)
            .and_then(|u| plan(&agent, u, tab, &url, &elements));
        let planned = match planned {
            Ok(planned) => planned,
            Err(PlanError::Mistake(error)) => {
                self.finish_call(task, &call, ToolOutcome::Failed { error });
                return QString::from("deny");
            }
            Err(PlanError::NoAddress) => {
                let why = "this page has no address an agent can use".to_owned();
                self.finish_call(task, &call, ToolOutcome::Denied { why });
                return QString::from("deny");
            }
        };
        let request = planned.request.clone();
        let step = planned.step.clone();
        if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&task) {
            running.plan = Some(planned);
            running.acted |= tool.acts();
        }

        // The person has the tab: wait for them rather than fail.
        if request.tab.is_some_and(is_taken_over) {
            if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&task) {
                running.paused = Some(call);
            }
            self.bump(task);
            return QString::from("paused");
        }

        let outcome = ion_safety::global().request_agent(request);
        match outcome.verdict() {
            Verdict::Allow => {
                if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&task) {
                    running.task.step_started(step);
                }
                self.bump(task);
                QString::from("allow")
            }
            Verdict::Ask => {
                if let (Some(prompt), Some(running)) = (
                    outcome.prompt,
                    self.as_mut().rust_mut().tasks.get_mut(&task),
                ) {
                    running.task.step_started(step);
                    running.pending = Some(Pending { call, prompt });
                }
                self.bump(task);
                QString::from("ask")
            }
            Verdict::Deny => {
                let why = lowercase_first(&outcome.decision.reason.to_string());
                self.finish_call(task, &call, ToolOutcome::Denied { why });
                QString::from("deny")
            }
        }
    }

    fn call_json(&self, task: i32, call: &QString) -> QString {
        let call = call.to_string();
        let Some(running) = self.tasks.get(&task) else {
            return QString::default();
        };
        let (Some((c, tool)), Some(plan)) = (&running.call, &running.plan) else {
            return QString::default();
        };
        if c.id != call {
            return QString::default();
        }
        let (element, text, url) = match &plan.call {
            Use::Click { element } => (Some(*element), None, None),
            Use::Fill { element, text } => (Some(*element), Some(text.as_str()), None),
            Use::GoTo { url } => (None, None, Some(url.as_str())),
            Use::ReadPage | Use::ListTabs => (None, None, None),
        };
        let value = json!({
            "tool": tool.name(),
            "element": element,
            "text": text,
            "url": url,
            // A click was approved as a click or as sending the form; the
            // page script refuses one that would do more than that.
            "allowSubmit": plan.request.action == ion_safety::Action::Submit,
        });
        QString::from(value.to_string().as_str())
    }

    fn answer(mut self: Pin<&mut Self>, task: i32, choice: &QString) {
        let Some(choice) = Choice::from_id(&choice.to_string()) else {
            return;
        };
        let Some(pending) = self
            .as_mut()
            .rust_mut()
            .tasks
            .get_mut(&task)
            .and_then(|r| r.pending.take())
        else {
            return;
        };
        let verdict = ion_safety::global()
            .answer(&pending.prompt, choice)
            .unwrap_or(Verdict::Deny);
        if verdict == Verdict::Allow {
            self.as_mut().bump(task);
            self.tool_approved(task, &QString::from(pending.call.as_str()));
        } else {
            let why = if choice == Choice::Deny {
                "you said no".to_owned()
            } else {
                "agents were stopped or the tab changed".to_owned()
            };
            self.finish_call(task, &pending.call, ToolOutcome::Denied { why });
        }
    }

    fn tool_result(
        mut self: Pin<&mut Self>,
        task: i32,
        call: &QString,
        ok: bool,
        payload: &QString,
    ) {
        let call = call.to_string();
        let Some((_, _, tab, agent)) = self.current_call(task, &call) else {
            return;
        };
        let Some(planned) = self.tasks.get(&task).and_then(|r| r.plan.clone()) else {
            return;
        };
        let payload: Value = serde_json::from_str(&payload.to_string()).unwrap_or(Value::Null);
        let text = |key: &str| payload[key].as_str().unwrap_or_default().to_owned();
        let mut new_elements = None;
        let mut logged_url = (planned.request.site.is_some()).then(|| text("url"));
        let outcome = if !ok {
            ToolOutcome::Failed {
                error: text("error"),
            }
        } else {
            match &planned.call {
                Use::ReadPage => {
                    let url = text("url");
                    let elements: Vec<Element> = payload["elements"]
                        .as_array()
                        .map(|list| list.iter().filter_map(Element::from_json).collect())
                        .unwrap_or_default();
                    let content = page_result(&url, &text("title"), &text("text"), &elements);
                    new_elements = Some(elements);
                    ToolOutcome::Done {
                        content,
                        step: planned.done_step(host_of(&url).as_deref()),
                    }
                }
                Use::ListTabs => {
                    let tabs: Vec<(String, String)> = payload["tabs"]
                        .as_array()
                        .map(|tabs| {
                            tabs.iter()
                                .map(|t| {
                                    (
                                        t["title"].as_str().unwrap_or_default().to_owned(),
                                        t["url"].as_str().unwrap_or_default().to_owned(),
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    ToolOutcome::Done {
                        content: tabs_result(&tabs),
                        step: planned.done_step(None),
                    }
                }
                Use::Click { element } => ToolOutcome::Done {
                    content: format!(
                        "{} [{element}]. Read the page again to see what changed.",
                        planned.done_step(None)
                    ),
                    step: planned.done_step(None),
                },
                Use::Fill { element, .. } => ToolOutcome::Done {
                    content: format!("{} [{element}].", planned.done_step(None)),
                    step: planned.done_step(None),
                },
                Use::GoTo { url } => {
                    // A new page: the old numbers mean nothing there.
                    new_elements = Some(Vec::new());
                    let landed = text("url");
                    logged_url = Some(url.clone());
                    ToolOutcome::Done {
                        content: format!("Opened {landed}. Use read_page to see what's there."),
                        step: planned.done_step(None),
                    }
                }
            }
        };
        ion_safety::global().log_action(ion_safety::ActionRecord {
            agent,
            action: planned.request.action.clone(),
            url: logged_url,
            tab: Some(tab),
            task: Some(task.to_string()),
            ok,
            detail: planned.detail.clone(),
            sensitive: planned.sensitive,
        });
        if let (Some(elements), Some(running)) =
            (new_elements, self.as_mut().rust_mut().tasks.get_mut(&task))
        {
            running.elements = elements;
        }
        self.finish_call(task, &call, outcome);
    }

    fn finish_call(mut self: Pin<&mut Self>, task: i32, call: &str, outcome: ToolOutcome) {
        if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&task) {
            running.call = None;
            running.plan = None;
            running.pending = None;
            running.paused = None;
            running.task.tool_finished(call, outcome);
        }
        self.pump(task);
    }

    fn take_over(mut self: Pin<&mut Self>, task: i32) {
        let Some(tab) = self.tasks.get(&task).map(|r| r.task.tab) else {
            return;
        };
        ion_safety::global().take_over(tab);
        self.as_mut().bump(task);
    }

    fn hand_back(mut self: Pin<&mut Self>, task: i32) {
        let Some(tab) = self.tasks.get(&task).map(|r| r.task.tab) else {
            return;
        };
        ion_safety::global().hand_back(tab);
        let paused = self
            .as_mut()
            .rust_mut()
            .tasks
            .get_mut(&task)
            .and_then(|r| r.paused.take());
        self.as_mut().bump(task);
        // Ask again: the page may have changed while the person had it.
        if let Some(call) = paused {
            let tool = self
                .tasks
                .get(&task)
                .and_then(|r| r.call.as_ref())
                .map(|(_, t)| *t);
            if let Some(tool) = tool {
                self.tool_requested(
                    task,
                    &QString::from(call.as_str()),
                    &QString::from(tool.name()),
                    tab as i32,
                );
            }
        }
    }

    fn stop(mut self: Pin<&mut Self>, task: i32) {
        if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&task) {
            running.task.stop();
            running.call = None;
            running.plan = None;
            running.pending = None;
            running.paused = None;
        }
        self.bump(task);
    }

    fn stop_all(mut self: Pin<&mut Self>) {
        let ids: Vec<i32> = self
            .tasks
            .iter()
            .filter(|(_, r)| !r.task.status().is_over())
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            self.as_mut().stop(id);
        }
    }

    fn answer_html(&self, markdown: &QString) -> QString {
        QString::from(ion_agent::markdown::to_html(&markdown.to_string()).as_str())
    }

    fn dismiss(mut self: Pin<&mut Self>, task: i32) {
        let over = self
            .tasks
            .get(&task)
            .is_some_and(|r| r.task.status().is_over());
        if over {
            self.as_mut().rust_mut().tasks.remove(&task);
            self.bump(task);
        }
    }
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}
