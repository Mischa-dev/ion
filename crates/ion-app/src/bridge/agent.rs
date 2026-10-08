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

        /// Ask `question` as a follow-up to `task` once it has answered (or
        /// failed or stopped), from its tab now showing `url` titled
        /// `title`. False while it is still working.
        #[qinvokable]
        #[cxx_name = "followUp"]
        fn follow_up(
            self: Pin<&mut Agent>,
            task: i32,
            question: &QString,
            url: &QString,
            title: &QString,
        ) -> bool;

        /// The task as JSON: `{id, agent, agentName, tab, question, status,
        /// steps: [{text, status}], answer, draft, error, prompt, acting,
        /// takenOver, opened, earlier}`, where `prompt` is `{text, detail, choices: [{id,
        /// label}]}` while waiting for the person, `draft` is the answer
        /// as far as the model has written it, `acting` says the agent
        /// has been changing the page, `takenOver` that the person took one
        /// of its tabs back, and `opened` lists the background tabs it
        /// opened as `[{tab, title}]`, and `earlier` holds the questions
        /// before this one as `[{question, steps, answer, error, status}]`.
        /// Empty for unknown ids.
        #[qinvokable]
        #[cxx_name = "taskJson"]
        fn task_json(self: &Agent, task: i32) -> QString;

        /// The newest task started on `tab` or working in it, or -1.
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
        /// element, text, url, after, allowSubmit}`, where `after` is the
        /// tab a new background tab goes after.
        #[qinvokable]
        #[cxx_name = "callJson"]
        fn call_json(self: &Agent, task: i32, call: &QString) -> QString;

        /// The person takes `tab` (the task's own or one it opened) back:
        /// the agent waits before its next step there until `handBack`.
        #[qinvokable]
        #[cxx_name = "takeOver"]
        fn take_over(self: Pin<&mut Agent>, task: i32, tab: i32);

        /// Hand every tab the person took from `task` back to it.
        #[qinvokable]
        #[cxx_name = "handBack"]
        fn hand_back(self: Pin<&mut Agent>, task: i32);

        /// Whether an agent is at work in `tab` (changing the page it was
        /// asked from, or in a background tab it opened), for the ring on
        /// the tab's icon.
        #[qinvokable]
        #[cxx_name = "actingOn"]
        fn acting_on(self: &Agent, tab: i32) -> bool;

        /// The person answered the task's prompt with a choice id.
        #[qinvokable]
        fn answer(self: Pin<&mut Agent>, task: i32, choice: &QString);

        /// A tool finished. `payload` is JSON: `{url, title, text,
        /// elements}` for read_page, `{tabs: [{title, url}]}` for
        /// list_tabs, `{url, title}` for go_to, `{tabId, url, title}` for
        /// open_tab, `{}` for click and fill, or
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
use std::time::{Duration, Instant};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use ion_agent::model::Model;
use ion_agent::openai::OpenAi;
use ion_agent::task::Context;
use ion_agent::tool::{
    Element, Plan, PlanError, Use, page_result, plan, tab_argument, tabs_result,
};
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
    /// The tab that call works on: the task's own or one it opened.
    target: u64,
    /// The checked call, once `authorize` has read it.
    plan: Option<Plan>,
    pending: Option<Pending>,
    /// The call waiting for the person to hand the tab back.
    paused: Option<String>,
    /// What the last read of each tab's page numbered.
    elements: BTreeMap<u64, Vec<Element>>,
    /// Background tabs the task opened, oldest first, with their titles.
    opened: Vec<(u64, String)>,
    /// The agent has been changing the page.
    acted: bool,
}

impl Running {
    /// The tab the task was asked from, then the tabs it opened.
    fn tabs(&self) -> impl Iterator<Item = u64> + '_ {
        std::iter::once(self.task.tab).chain(self.opened.iter().map(|t| t.0))
    }
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

/// Agents that can be asked from the address bar, as (id, name): Ion
/// Agent, then each `[agents.<id>]` with a model of its own.
fn askable() -> Vec<(String, String)> {
    let config = ion_config::global().config();
    let safety = ion_safety::global();
    let mut agents = vec![(
        invoke::DEFAULT_AGENT.to_owned(),
        safety.policy().display_name(invoke::DEFAULT_AGENT),
    )];
    for (id, profile) in &config.agents.profiles {
        if !profile.model.trim().is_empty()
            && id != invoke::DEFAULT_AGENT
            && ion_safety::policy::is_valid_id(id)
        {
            agents.push((id.clone(), safety.policy().display_name(id)));
        }
    }
    agents
}

/// The model `agent` runs on, with its key from the environment: its own
/// `[agents.<id>]` model, or `[ai]` for Ion Agent.
fn configured_model(agent: &str) -> Result<Arc<dyn Model>, String> {
    let config = ion_config::global().config();
    let ai = config.ai.clone();
    if ai.provider != "openai" {
        return Err(format!(
            "Ion Agent can't use the {:?} provider yet. Set ai.provider to \"openai\".",
            ai.provider
        ));
    }
    let own = config
        .agents
        .profiles
        .get(agent)
        .filter(|p| !p.model.trim().is_empty());
    let (model, base_url, key_env) = match own {
        Some(p) => (
            p.model.trim().to_owned(),
            if p.base_url.trim().is_empty() {
                ai.base_url.clone()
            } else {
                p.base_url.trim().to_owned()
            },
            p.api_key_env
                .clone()
                .unwrap_or_else(|| ai.api_key_env.clone()),
        ),
        None if agent == invoke::DEFAULT_AGENT => (
            ai.model.clone(),
            ai.base_url.clone(),
            ai.api_key_env.clone(),
        ),
        None => {
            return Err(format!(
                "There's no agent called @{agent}. Give [agents.{agent}] a model in Ion's config to ask it."
            ));
        }
    };
    let key = if key_env.trim().is_empty() {
        None
    } else {
        match std::env::var(key_env.trim()) {
            Ok(key) if !key.trim().is_empty() => Some(key),
            _ => {
                return Err(format!(
                    "No API key: set {} in Ion's environment, or set its apiKeyEnv to \"\" for a local model.",
                    key_env.trim()
                ));
            }
        }
    };
    Ok(Arc::new(OpenAi::new(&base_url, &model, key)))
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
            askable()
                .into_iter()
                .filter(|(id, _)| id.starts_with(prefix))
                .map(|(id, name)| {
                    json!({
                        "kind": "agent",
                        "title": name,
                        "subtitle": format!("@{id} · ask about this page or get something done on it"),
                        "hint": "Tab",
                        "action": "complete",
                        "value": format!("@{id} "),
                    })
                })
                .collect()
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
        let name = ion_safety::global().policy().display_name(&ask.agent);
        let instructions = ion_config::global()
            .config()
            .agents
            .profiles
            .get(&ask.agent)
            .map(|p| p.instructions.clone())
            .unwrap_or_default();
        let task = Task::new(id as u64, &ask.agent, tab_id, &ask.question, &context)
            .with_persona(&name, &instructions);
        // Asking from a tab hands it to the agent, even if the person took
        // it back from an earlier task.
        ion_safety::global().hand_back(tab_id);
        self.as_mut().rust_mut().tasks.insert(
            id,
            Running {
                task,
                call: None,
                target: tab_id,
                plan: None,
                pending: None,
                paused: None,
                elements: BTreeMap::new(),
                opened: Vec::new(),
                acted: false,
            },
        );
        self.as_mut().pump(id);
        id
    }

    fn follow_up(
        mut self: Pin<&mut Self>,
        task: i32,
        question: &QString,
        url: &QString,
        title: &QString,
    ) -> bool {
        let question = question.to_string();
        let question = question.trim();
        if question.is_empty() || !enabled() {
            return false;
        }
        let context = Context {
            title: title.to_string(),
            url: url.to_string(),
        };
        let mut tabs = None;
        if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&task) {
            if running.task.follow_up(question, &context) {
                tabs = Some(running.tabs().collect::<Vec<u64>>());
            }
        }
        let Some(tabs) = tabs else {
            return false;
        };
        // Asking again hands the agent its tabs again, like asking first.
        {
            let mut safety = ion_safety::global();
            for tab in tabs {
                safety.hand_back(tab);
            }
        }
        self.pump(task);
        true
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
                    let agent = self
                        .tasks
                        .get(&id)
                        .map(|r| r.task.agent.clone())
                        .unwrap_or_default();
                    match configured_model(&agent) {
                        Ok(model) => {
                            let thread = self.qt_thread();
                            std::thread::spawn(move || {
                                // Show the answer as it is written, a few
                                // times a second rather than per token.
                                let mut unsent = String::new();
                                let mut last = Instant::now();
                                let mut show = |piece: &str| {
                                    unsent.push_str(piece);
                                    if last.elapsed() >= Duration::from_millis(60) {
                                        let text = std::mem::take(&mut unsent);
                                        let _ = thread.queue(move |obj| obj.model_text(id, text));
                                        last = Instant::now();
                                    }
                                };
                                let reply = model.reply_streaming(&messages, &Tool::ALL, &mut show);
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
                    let mut requested = None;
                    if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&id) {
                        // Agents work on the tab they were asked from and
                        // the tabs they opened, nothing else.
                        let home = running.task.tab;
                        let target = match tab_argument(tool, &call.arguments) {
                            Ok(None) => Ok(home),
                            Ok(Some(tab))
                                if tab == home || running.opened.iter().any(|t| t.0 == tab) =>
                            {
                                Ok(tab)
                            }
                            Ok(Some(tab)) => Err(format!(
                                "tab {tab} isn't one you opened; use the person's tab or open_tab"
                            )),
                            Err(error) => Err(error),
                        };
                        match target {
                            Ok(target) => {
                                requested = Some((QString::from(call.id.as_str()), target));
                                running.call = Some((call, tool));
                                running.target = target;
                                running.plan = None;
                            }
                            Err(error) => running
                                .task
                                .tool_finished(&call.id, ToolOutcome::Failed { error }),
                        }
                    }
                    let Some((call_id, target)) = requested else {
                        continue;
                    };
                    self.as_mut().bump(id);
                    self.as_mut().tool_requested(
                        id,
                        &call_id,
                        &QString::from(tool.name()),
                        target as i32,
                    );
                    return;
                }
                Next::Idle | Next::Finished => break,
            }
        }
        self.bump(id);
    }

    fn model_text(mut self: Pin<&mut Self>, id: i32, text: String) {
        if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&id) {
            running.task.model_text(&text);
            self.bump(id);
        }
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
        value["acting"] = Value::from((running.acted || !running.opened.is_empty()) && !over);
        value["takenOver"] = Value::from(!over && running.tabs().any(is_taken_over));
        value["opened"] = Value::from(
            running
                .opened
                .iter()
                .map(|t| json!({ "tab": t.0, "title": t.1 }))
                .collect::<Vec<_>>(),
        );
        QString::from(value.to_string().as_str())
    }

    fn latest_task(&self, tab: i32) -> i32 {
        let Ok(tab) = u64::try_from(tab) else {
            return -1;
        };
        self.tasks
            .iter()
            .rev()
            .find(|(_, r)| r.tabs().any(|t| t == tab))
            .map_or(-1, |(id, _)| *id)
    }

    fn acting_on(&self, tab: i32) -> bool {
        let Ok(tab) = u64::try_from(tab) else {
            return false;
        };
        !is_taken_over(tab)
            && self.tasks.values().any(|r| {
                !r.task.status().is_over()
                    && ((r.task.tab == tab && r.acted) || r.opened.iter().any(|t| t.0 == tab))
            })
    }

    /// The running call `call` of `task`, if that is what's running, with
    /// the tab it works on.
    fn current_call(&self, task: i32, call: &str) -> Option<(ToolCall, Tool, u64, String)> {
        let running = self.tasks.get(&task)?;
        let (c, tool) = running.call.as_ref()?;
        (c.id == call).then(|| (c.clone(), *tool, running.target, running.task.agent.clone()))
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
            .and_then(|r| r.elements.get(&tab).cloned())
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
            Use::GoTo { url } | Use::OpenTab { url } => (None, None, Some(url.as_str())),
            Use::ReadPage | Use::ListTabs => (None, None, None),
        };
        let value = json!({
            "tool": tool.name(),
            "element": element,
            "text": text,
            "url": url,
            // New background tabs go after the task's other tabs, so they
            // sit together in the strip.
            "after": running.opened.last().map_or(running.task.tab, |t| t.0),
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
        let mut opened_tab = None;
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
                    let mut content = page_result(&url, &text("title"), &text("text"), &elements);
                    if self.tasks.get(&task).is_some_and(|r| r.task.tab != tab) {
                        content = format!("Tab {tab}:\n{content}");
                    }
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
                Use::OpenTab { url } => {
                    logged_url = Some(url.clone());
                    match payload["tabId"].as_u64() {
                        Some(new_tab) => {
                            opened_tab = Some((new_tab, text("title")));
                            ToolOutcome::Done {
                                content: format!(
                                    "Opened tab {new_tab}: {}. Pass \"tab\": {new_tab} to \
                                     read_page, click, fill and go_to to work there.",
                                    text("url")
                                ),
                                step: planned.done_step(None),
                            }
                        }
                        None => ToolOutcome::Failed {
                            error: "the tab didn't open".to_owned(),
                        },
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
        if let Some(running) = self.as_mut().rust_mut().tasks.get_mut(&task) {
            if let Some(elements) = new_elements {
                running.elements.insert(tab, elements);
            }
            if let Some(opened) = opened_tab {
                running.opened.push(opened);
            }
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

    fn take_over(mut self: Pin<&mut Self>, task: i32, tab: i32) {
        let Ok(tab) = u64::try_from(tab) else {
            return;
        };
        let owns = self
            .tasks
            .get(&task)
            .is_some_and(|r| r.tabs().any(|t| t == tab));
        if owns {
            ion_safety::global().take_over(tab);
            self.as_mut().bump(task);
        }
    }

    fn hand_back(mut self: Pin<&mut Self>, task: i32) {
        let Some(running) = self.tasks.get(&task) else {
            return;
        };
        let tabs: Vec<u64> = running.tabs().collect();
        let target = running.target;
        {
            let mut safety = ion_safety::global();
            for tab in tabs {
                safety.hand_back(tab);
            }
        }
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
                    target as i32,
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
