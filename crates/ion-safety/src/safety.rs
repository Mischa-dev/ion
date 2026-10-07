//! [`Safety`]: the policy plus remembered rules, prompts and the activity log,
//! behind one object.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::audit::{self, AuditLog, Entry, Kind};
use crate::capability::{Action, SiteCapability};
use crate::policy::{AgentProfile, AgentRequest, Decision, Policy, Reason, SiteRequest, Verdict};
use crate::prompt::{Choice, Prompt, Subject};
use crate::rule::{Effect, Lifetime, Rule, Source, What, Who};
use crate::site::{Origin, SitePattern};
use crate::store::RuleStore;

/// The answer to a request: a decision, and a prompt to show when it is
/// "ask".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub decision: Decision,
    pub prompt: Option<Prompt>,
}

impl Outcome {
    pub fn verdict(&self) -> Verdict {
        self.decision.verdict
    }
}

/// Something an agent did, for the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRecord {
    pub agent: String,
    pub action: Action,
    /// The page's URL (query and fragment are dropped before logging).
    pub url: Option<String>,
    pub tab: Option<u64>,
    pub task: Option<String>,
    pub ok: bool,
    /// What happened, in a few words ("clicked “Sign in”", "typed into
    /// Search").
    pub detail: String,
    /// True when `detail` holds what was typed into a password or
    /// personal-data field; only its length is logged.
    pub sensitive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerError {
    /// The prompt does not offer that choice.
    NotOffered,
}

type Clock = Box<dyn Fn() -> u64 + Send>;

fn system_clock() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub struct Safety {
    policy: Policy,
    /// How many times each tab has closed. A reopened tab keeps its id, so
    /// prompts remember this count to tell its lives apart.
    tab_closures: BTreeMap<u64, u32>,
    store: Option<RuleStore>,
    audit: Option<AuditLog>,
    clock: Clock,
}

impl Default for Safety {
    fn default() -> Self {
        Safety::in_memory()
    }
}

impl Safety {
    /// Nothing saved, nothing logged (tests, and when there is no data dir).
    pub fn in_memory() -> Safety {
        Safety {
            policy: Policy::new(),
            tab_closures: BTreeMap::new(),
            store: None,
            audit: None,
            clock: Box::new(system_clock),
        }
    }

    /// Keep remembered rules in `dir/rules.json` and the log in `dir/audit/`,
    /// pruning log files older than `retention_days`. An unreadable rules
    /// file is set aside and Ion starts with none, so everything asks again
    /// rather than anything being allowed.
    pub fn open(dir: &Path, retention_days: u32) -> Safety {
        let store = RuleStore::new(dir.join("rules.json"));
        let mut audit = AuditLog::new(dir.join("audit"));
        audit.set_retention_days(retention_days);
        let mut safety = Safety {
            policy: Policy::new(),
            tab_closures: BTreeMap::new(),
            store: None,
            audit: Some(audit),
            clock: Box::new(system_clock),
        };
        match store.load() {
            Ok(rules) => rules.into_iter().for_each(|r| {
                safety.policy.add_user_rule(r);
            }),
            Err(e) => {
                eprintln!("ion: can't read {}: {e}", store.path().display());
                match store.set_aside() {
                    Ok(aside) => eprintln!("ion: moved it to {}", aside.display()),
                    Err(e) => eprintln!("ion: can't move it aside: {e}"),
                }
            }
        }
        safety.store = Some(store);
        safety.prune_log();
        safety
    }

    /// Use `clock` (unix seconds) instead of the system clock.
    pub fn with_clock(mut self, clock: impl Fn() -> u64 + Send + 'static) -> Safety {
        self.clock = Box::new(clock);
        self
    }

    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    fn now(&self) -> u64 {
        (self.clock)()
    }

    fn log(&self, entry: Entry) {
        if let Some(Err(e)) = self.audit.as_ref().map(|audit| audit.append(&entry)) {
            eprintln!("ion: can't write the activity log: {e}");
        }
    }

    fn save(&self) {
        if let Some(store) = &self.store {
            let rules = self.policy.user_rules();
            if let Err(e) = store.save(rules) {
                eprintln!(
                    "ion: can't save permissions to {}: {e}",
                    store.path().display()
                );
            }
        }
    }

    fn prune_log(&self) {
        if let Some(Err(e)) = self.audit.as_ref().map(|audit| audit.prune(self.now())) {
            eprintln!("ion: can't prune the activity log: {e}");
        }
    }

    /// Agent profiles from config. Rules the person made are kept.
    pub fn set_profiles(&mut self, profiles: BTreeMap<String, AgentProfile>) {
        self.policy.set_profiles(profiles);
    }

    pub fn set_audit_retention_days(&mut self, days: u32) {
        if let Some(audit) = &mut self.audit {
            audit.set_retention_days(days);
        }
        self.prune_log();
    }

    /// A page asks for a capability. Not logged unless the person answers:
    /// pages ask often, and the answers are what matter.
    pub fn request_site(&self, request: SiteRequest) -> Outcome {
        let decision = self.policy.decide_site(&request);
        let generation = self.tab_generation(request.tab);
        let prompt = (decision.verdict == Verdict::Ask).then(|| Prompt {
            tab_generation: generation,
            ..Prompt::for_site(request)
        });
        Outcome { decision, prompt }
    }

    /// An agent asks to do something. Every decision is logged.
    pub fn request_agent(&self, request: AgentRequest) -> Outcome {
        let decision = self.policy.decide_agent(&request);
        let mut entry = agent_entry(self.now(), Kind::Decision, &request);
        entry.verdict = Some(decision.verdict);
        entry.reason = Some(decision.reason.to_string());
        self.log(entry);
        let generation = self.tab_generation(request.tab);
        let prompt = (decision.verdict == Verdict::Ask).then(|| {
            let name = self.policy.display_name(&request.agent);
            Prompt {
                tab_generation: generation,
                ..Prompt::for_agent(request, &name)
            }
        });
        Outcome { decision, prompt }
    }

    fn tab_generation(&self, tab: Option<u64>) -> u32 {
        tab.and_then(|t| self.tab_closures.get(&t).copied())
            .unwrap_or(0)
    }

    /// Why an answer to `prompt` no longer counts: its tab closed, or (for
    /// agents) all agents were stopped, the agent paused or the tab taken
    /// back while the prompt was up.
    fn overridden(&self, prompt: &Prompt) -> Option<Reason> {
        let tab = match &prompt.subject {
            Subject::Agent(request) => request.tab,
            Subject::Site(request) => request.tab,
        };
        if tab.is_some() && self.tab_generation(tab) != prompt.tab_generation {
            return Some(Reason::TabClosed);
        }
        let Subject::Agent(request) = &prompt.subject else {
            return None;
        };
        // Anything that now denies the request (including a rule added by a
        // config reload while the prompt was up) wins over the answer.
        let decision = self.policy.decide_agent(request);
        (decision.verdict == Verdict::Deny).then_some(decision.reason)
    }

    /// The person answered `prompt`. Remembers the answer as the prompt
    /// says, logs it, and returns the verdict for the waiting request.
    ///
    /// The policy is checked again first: if the tab closed, agents were
    /// stopped, the agent paused or the tab taken back while the prompt was
    /// up, the request is denied whatever the answer, and nothing is
    /// remembered.
    pub fn answer(&mut self, prompt: &Prompt, choice: Choice) -> Result<Verdict, AnswerError> {
        let now = self.now();
        let (mut verdict, mut rule) = prompt.resolve(choice, now).ok_or(AnswerError::NotOffered)?;
        let overridden = self.overridden(prompt);
        if overridden.is_some() {
            verdict = Verdict::Deny;
            rule = None;
        }
        let mut entry = match &prompt.subject {
            Subject::Agent(request) => agent_entry(now, Kind::Answer, request),
            Subject::Site(request) => {
                let mut e = Entry::new(now, Kind::Answer);
                e.capability = Some(request.capability);
                e.target = Some(request.origin.to_string());
                e.tab = request.tab;
                e
            }
        };
        entry.verdict = Some(verdict);
        entry.reason = overridden.map(|r| r.to_string());
        entry.detail = Some(choice.id().to_owned());
        self.log(entry);
        if let Some(rule) = rule {
            if self.policy.add_user_rule(rule) {
                self.save();
            }
        }
        Ok(verdict)
    }

    /// The person answered the site prompt shown for `request`, made when
    /// the tab's generation was `tab_generation` (from the shown prompt). The
    /// prompt is rebuilt from the request: if the tab closed since, the
    /// answer denies; if another answer decided the request while it was up
    /// (the same prompt in a second tab), a "Block" is still honored and
    /// remembered, and any other choice gets the current verdict.
    pub fn answer_site(
        &mut self,
        request: SiteRequest,
        tab_generation: u32,
        choice: Choice,
    ) -> Result<Verdict, AnswerError> {
        let outcome = self.request_site(request.clone());
        let prompt = match outcome.prompt {
            Some(prompt) => prompt,
            None if choice == Choice::Deny => Prompt::for_site(request),
            None => return Ok(outcome.decision.verdict),
        };
        let prompt = Prompt {
            tab_generation,
            ..prompt
        };
        self.answer(&prompt, choice)
    }

    /// Record what an agent did.
    pub fn log_action(&self, record: ActionRecord) {
        let mut entry = Entry::new(self.now(), Kind::Action);
        entry.agent = Some(record.agent);
        entry.target = match &record.action {
            Action::UseConnector(name) => Some(name.clone()),
            _ => record.url.as_deref().map(crate::site::loggable_url),
        };
        entry.action = Some(record.action);
        entry.tab = record.tab;
        entry.task = record.task;
        entry.ok = Some(record.ok);
        entry.detail = Some(audit::redact(&record.detail, record.sensitive));
        self.log(entry);
    }

    /// Stop every agent until [`Safety::resume_all`].
    pub fn stop_all(&mut self) {
        if !self.policy.is_stopped() {
            self.policy.set_stopped(true);
            self.log(Entry::new(self.now(), Kind::Stop));
        }
    }

    pub fn resume_all(&mut self) {
        if self.policy.is_stopped() {
            self.policy.set_stopped(false);
            self.log(Entry::new(self.now(), Kind::Resume));
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.policy.is_stopped()
    }

    pub fn set_agent_paused(&mut self, agent: &str, paused: bool) {
        self.policy.set_paused(agent, paused);
    }

    /// The person takes `tab` back: agents may not touch it until handed back.
    pub fn take_over(&mut self, tab: u64) {
        if !self.policy.is_taken_over(tab) {
            self.policy.set_taken_over(tab, true);
            let mut entry = Entry::new(self.now(), Kind::TakeOver);
            entry.tab = Some(tab);
            self.log(entry);
        }
    }

    pub fn hand_back(&mut self, tab: u64) {
        if self.policy.is_taken_over(tab) {
            self.policy.set_taken_over(tab, false);
            let mut entry = Entry::new(self.now(), Kind::HandBack);
            entry.tab = Some(tab);
            self.log(entry);
        }
    }

    pub fn tab_closed(&mut self, tab: u64) {
        *self.tab_closures.entry(tab).or_default() += 1;
        self.policy.tab_closed(tab);
    }

    /// The person's remembered site permissions, for a settings page.
    pub fn site_permissions(&self) -> Vec<&Rule> {
        self.policy
            .user_rules()
            .iter()
            .filter(|r| r.who == Who::Site && r.lifetime == Lifetime::Forever)
            .collect()
    }

    /// Remembered rules for one agent (forever and session), for a settings
    /// page.
    pub fn agent_rules(&self, agent: &str) -> Vec<&Rule> {
        self.policy
            .user_rules()
            .iter()
            .filter(|r| matches!(&r.who, Who::Agent(id) if id == agent))
            .collect()
    }

    /// Forget a rule the person made. Config rules can't be forgotten here.
    /// Returns whether anything was removed.
    pub fn forget(&mut self, rule: &Rule) -> bool {
        if rule.source != Source::User {
            return false;
        }
        let removed = self.policy.retain_user_rules(|r| r != rule) > 0;
        if removed {
            let mut entry = Entry::new(self.now(), Kind::Forget);
            if let Who::Agent(id) = &rule.who {
                entry.agent = Some(id.clone());
            }
            entry.target = Some(rule.site.to_string());
            entry.detail = Some(format!("{:?} {:?}", rule.what, rule.effect));
            self.log(entry);
            self.save();
        }
        removed
    }

    /// Forget every site permission remembered for `origin`.
    pub fn forget_site(&mut self, origin: &Origin) -> usize {
        let rules: Vec<Rule> = self
            .site_permissions()
            .into_iter()
            .filter(|r| r.site.matches(Some(origin)))
            .cloned()
            .collect();
        rules.iter().filter(|r| self.forget(r)).count()
    }

    /// What the person chose for `origin`, one entry per capability, for the
    /// toolbar's site permissions panel.
    pub fn site_permissions_for(&self, origin: &Origin) -> Vec<(SiteCapability, Effect)> {
        self.site_permissions()
            .into_iter()
            .filter(|r| r.site.matches(Some(origin)))
            .filter_map(|r| match r.what {
                What::Site(capability) => Some((capability, r.effect)),
                _ => None,
            })
            .collect()
    }

    /// Remember Allow or Block for `capability` on `origin`, replacing what
    /// was remembered before, as the panel's switch does. Pages without a
    /// site of their own (local files, `data:`) can't remember anything, so
    /// this returns false for them.
    pub fn set_site_permission(
        &mut self,
        origin: &Origin,
        capability: SiteCapability,
        allowed: bool,
    ) -> bool {
        if !origin.is_distinct() {
            return false;
        }
        let what = What::Site(capability);
        self.policy.retain_user_rules(|r| {
            !(r.who == Who::Site && r.what == what && r.site.matches(Some(origin)))
        });
        let effect = if allowed { Effect::Allow } else { Effect::Deny };
        let now = self.now();
        self.policy.add_user_rule(Rule {
            who: Who::Site,
            what,
            site: SitePattern::Origin(origin.clone()),
            effect,
            lifetime: Lifetime::Forever,
            source: Source::User,
            created: now,
        });
        let mut entry = Entry::new(now, Kind::Answer);
        entry.capability = Some(capability);
        entry.target = Some(origin.to_string());
        entry.verdict = Some(if allowed {
            Verdict::Allow
        } else {
            Verdict::Deny
        });
        entry.detail = Some("site permissions panel".to_owned());
        self.log(entry);
        self.save();
        true
    }

    /// Forget what was remembered for `capability` on `origin`, so the site
    /// asks again. Returns whether anything was forgotten.
    pub fn forget_site_permission(&mut self, origin: &Origin, capability: SiteCapability) -> bool {
        let rules: Vec<Rule> = self
            .site_permissions()
            .into_iter()
            .filter(|r| r.what == What::Site(capability) && r.site.matches(Some(origin)))
            .cloned()
            .collect();
        rules.iter().filter(|r| self.forget(r)).count() > 0
    }

    /// Log entries at or after `since` (unix seconds), oldest first.
    pub fn activity_since(&self, since: u64) -> io::Result<Vec<Entry>> {
        match &self.audit {
            Some(audit) => audit.read_since(since),
            None => Ok(Vec::new()),
        }
    }
}

fn agent_entry(time: u64, kind: Kind, request: &AgentRequest) -> Entry {
    let mut entry = Entry::new(time, kind);
    entry.agent = Some(request.agent.clone());
    entry.target = match &request.action {
        Action::UseConnector(name) => Some(name.clone()),
        _ => request.site.as_ref().map(Origin::to_string),
    };
    entry.action = Some(request.action.clone());
    entry.tab = request.tab;
    entry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_dir;
    use crate::policy::{Reason, TrustLevel};

    const NOW: u64 = 1_791_367_200;

    fn origin(s: &str) -> Origin {
        Origin::parse(s).unwrap()
    }

    fn agent_req(action: Action, site: &str) -> AgentRequest {
        AgentRequest {
            agent: "ion".into(),
            action,
            site: Some(origin(site)),
            tab: Some(1),
        }
    }

    fn camera(site: &str) -> SiteRequest {
        SiteRequest {
            origin: origin(site),
            capability: SiteCapability::Camera,
            tab: Some(1),
        }
    }

    #[test]
    fn a_later_block_wins_over_an_earlier_allow() {
        let mut safety = Safety::in_memory().with_clock(|| NOW);
        let meet = "https://meet.example";
        // The same prompt is up in two tabs; the first allows.
        assert_eq!(
            safety.answer_site(camera(meet), 0, Choice::Allow),
            Ok(Verdict::Allow)
        );
        let mut other_tab = camera(meet);
        other_tab.tab = Some(2);
        assert_eq!(
            safety.answer_site(other_tab.clone(), 0, Choice::AllowOnce),
            Ok(Verdict::Allow)
        );
        assert_eq!(
            safety.answer_site(other_tab, 0, Choice::Deny),
            Ok(Verdict::Deny)
        );
        assert_eq!(safety.request_site(camera(meet)).verdict(), Verdict::Deny);
    }

    #[test]
    fn site_answers_after_the_tab_closed_deny() {
        let mut safety = Safety::in_memory().with_clock(|| NOW);
        let meet = "https://meet.example";
        let shown = safety.request_site(camera(meet)).prompt.unwrap();
        safety.tab_closed(1);
        assert_eq!(
            safety.answer_site(camera(meet), shown.tab_generation, Choice::Allow),
            Ok(Verdict::Deny)
        );
        assert_eq!(safety.request_site(camera(meet)).verdict(), Verdict::Ask);
    }

    #[test]
    fn replacing_a_saved_rule_with_a_session_one_saves() {
        let dir = test_dir("safety-replace");
        let gh = "https://github.com";
        {
            let mut safety = Safety::open(&dir, 30).with_clock(|| NOW);
            let first = safety
                .request_agent(agent_req(Action::Submit, gh))
                .prompt
                .unwrap();
            let second = first.clone();
            assert_eq!(safety.answer(&first, Choice::Allow), Ok(Verdict::Allow));
            assert_eq!(safety.answer(&second, Choice::Deny), Ok(Verdict::Deny));
        }
        let safety = Safety::open(&dir, 30).with_clock(|| NOW);
        assert_eq!(
            safety
                .request_agent(agent_req(Action::Submit, gh))
                .verdict(),
            Verdict::Ask
        );
    }

    #[test]
    fn site_answers_persist_across_restarts() {
        let dir = test_dir("safety-site");
        {
            let mut safety = Safety::open(&dir, 30).with_clock(|| NOW);
            let outcome = safety.request_site(camera("https://meet.example"));
            assert_eq!(outcome.verdict(), Verdict::Ask);
            let prompt = outcome.prompt.unwrap();
            assert_eq!(safety.answer(&prompt, Choice::Allow), Ok(Verdict::Allow));
            assert_eq!(
                safety
                    .request_site(camera("https://meet.example"))
                    .verdict(),
                Verdict::Allow
            );
        }
        let mut safety = Safety::open(&dir, 30).with_clock(|| NOW);
        assert_eq!(
            safety
                .request_site(camera("https://meet.example"))
                .verdict(),
            Verdict::Allow
        );
        assert_eq!(safety.site_permissions().len(), 1);
        assert_eq!(safety.forget_site(&origin("https://meet.example/x")), 1);
        assert_eq!(
            safety
                .request_site(camera("https://meet.example"))
                .verdict(),
            Verdict::Ask
        );
        let safety = Safety::open(&dir, 30);
        assert!(safety.site_permissions().is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_panel_switches_and_forgets_one_permission() {
        let dir = test_dir("safety-panel");
        let meet = origin("https://meet.example");
        {
            let mut safety = Safety::open(&dir, 30).with_clock(|| NOW);
            let prompt = safety
                .request_site(camera("https://meet.example"))
                .prompt
                .unwrap();
            assert_eq!(safety.answer(&prompt, Choice::Allow), Ok(Verdict::Allow));
            assert!(safety.set_site_permission(&meet, SiteCapability::Location, true));
            assert!(safety.set_site_permission(&meet, SiteCapability::Camera, false));
            let mut stored = safety.site_permissions_for(&meet);
            stored.sort_by_key(|(c, _)| c.to_qt());
            assert_eq!(
                stored,
                vec![
                    (SiteCapability::Camera, Effect::Deny),
                    (SiteCapability::Location, Effect::Allow)
                ]
            );
            assert!(
                safety
                    .site_permissions_for(&origin("https://other.example"))
                    .is_empty()
            );
        }
        let mut safety = Safety::open(&dir, 30).with_clock(|| NOW);
        assert_eq!(
            safety
                .request_site(camera("https://meet.example"))
                .verdict(),
            Verdict::Deny
        );
        assert!(safety.forget_site_permission(&meet, SiteCapability::Camera));
        assert!(!safety.forget_site_permission(&meet, SiteCapability::Camera));
        assert_eq!(
            safety
                .request_site(camera("https://meet.example"))
                .verdict(),
            Verdict::Ask
        );
        assert_eq!(safety.site_permissions_for(&meet).len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_panel_cannot_remember_for_local_files() {
        let mut safety = Safety::in_memory();
        let file = origin("file:///home/me/page.html");
        assert!(!safety.set_site_permission(&file, SiteCapability::Camera, true));
        assert!(safety.site_permissions().is_empty());
    }

    #[test]
    fn allow_once_is_not_remembered() {
        let mut safety = Safety::in_memory();
        let prompt = safety
            .request_site(camera("https://meet.example"))
            .prompt
            .unwrap();
        assert_eq!(
            safety.answer(&prompt, Choice::AllowOnce),
            Ok(Verdict::Allow)
        );
        assert_eq!(
            safety
                .request_site(camera("https://meet.example"))
                .verdict(),
            Verdict::Ask
        );
    }

    #[test]
    fn choices_a_prompt_does_not_offer_are_refused() {
        let mut safety = Safety::in_memory();
        let prompt = safety
            .request_agent(agent_req(Action::Purchase, "https://shop.example"))
            .prompt
            .unwrap();
        assert_eq!(
            safety.answer(&prompt, Choice::Allow),
            Err(AnswerError::NotOffered)
        );
        assert!(safety.policy().user_rules().is_empty());
    }

    #[test]
    fn agent_flow_is_logged_end_to_end() {
        let dir = test_dir("safety-agent");
        let mut safety = Safety::open(&dir, 30).with_clock(|| NOW);
        let outcome = safety.request_agent(agent_req(Action::ReadPage, "https://github.com/a?q=1"));
        assert_eq!(outcome.verdict(), Verdict::Ask);
        let prompt = outcome.prompt.unwrap();
        assert_eq!(prompt.text, "Ion Agent wants to read github.com");
        safety.answer(&prompt, Choice::Allow).unwrap();
        assert_eq!(
            safety
                .request_agent(agent_req(Action::Interact, "https://github.com"))
                .verdict(),
            Verdict::Allow
        );
        safety.log_action(ActionRecord {
            agent: "ion".into(),
            action: Action::Interact,
            url: Some("https://github.com/login?next=/secret#x".into()),
            tab: Some(1),
            task: Some("t1".into()),
            ok: true,
            detail: "hunter2".into(),
            sensitive: true,
        });
        safety.stop_all();
        safety.stop_all(); // Logged once.
        assert_eq!(
            safety
                .request_agent(agent_req(Action::Interact, "https://github.com"))
                .decision
                .reason,
            Reason::Stopped
        );
        safety.resume_all();

        let log = safety.activity_since(0).unwrap();
        let kinds: Vec<Kind> = log.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            [
                Kind::Decision,
                Kind::Answer,
                Kind::Decision,
                Kind::Action,
                Kind::Stop,
                Kind::Decision,
                Kind::Resume
            ]
        );
        assert_eq!(log[0].target.as_deref(), Some("https://github.com"));
        assert_eq!(log[0].verdict, Some(Verdict::Ask));
        assert_eq!(log[3].target.as_deref(), Some("https://github.com/login"));
        assert_eq!(log[3].detail.as_deref(), Some("[redacted, 7 chars]"));
        assert_eq!(log[5].verdict, Some(Verdict::Deny));
        let text = std::fs::read_to_string(dir.join("audit/2026-10-07.jsonl")).unwrap();
        assert!(!text.contains("hunter2"));
        assert!(!text.contains("secret"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn agent_denials_do_not_survive_a_restart() {
        let dir = test_dir("safety-deny");
        {
            let mut safety = Safety::open(&dir, 30);
            let prompt = safety
                .request_agent(agent_req(Action::Interact, "https://github.com"))
                .prompt
                .unwrap();
            assert_eq!(safety.answer(&prompt, Choice::Deny), Ok(Verdict::Deny));
            assert_eq!(
                safety
                    .request_agent(agent_req(Action::Interact, "https://github.com"))
                    .verdict(),
                Verdict::Deny
            );
        }
        let safety = Safety::open(&dir, 30);
        assert_eq!(
            safety
                .request_agent(agent_req(Action::Interact, "https://github.com"))
                .verdict(),
            Verdict::Ask
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn take_over_blocks_one_tab() {
        let mut safety = Safety::in_memory();
        safety.set_profiles(
            [(
                "ion".to_owned(),
                AgentProfile {
                    trust: TrustLevel::Full,
                    ..AgentProfile::default()
                },
            )]
            .into(),
        );
        safety.take_over(1);
        let req = agent_req(Action::Interact, "https://github.com");
        assert_eq!(
            safety.request_agent(req.clone()).decision.reason,
            Reason::TakenOver
        );
        safety.hand_back(1);
        assert_eq!(safety.request_agent(req).verdict(), Verdict::Allow);
    }

    #[test]
    fn an_unreadable_rules_file_means_everything_asks() {
        let dir = test_dir("safety-corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("rules.json"), "garbage").unwrap();
        let safety = Safety::open(&dir, 30);
        assert!(safety.policy().user_rules().is_empty());
        assert!(dir.join("rules.json.unreadable").exists());
        assert_eq!(
            safety.request_site(camera("https://a.example")).verdict(),
            Verdict::Ask
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn answers_after_a_stop_or_take_over_deny() {
        let mut safety = Safety::in_memory();
        let req = agent_req(Action::Interact, "https://github.com");
        let prompt = safety.request_agent(req.clone()).prompt.unwrap();
        safety.stop_all();
        assert_eq!(safety.answer(&prompt, Choice::Allow), Ok(Verdict::Deny));
        assert!(safety.policy().user_rules().is_empty());
        safety.resume_all();

        safety.take_over(1);
        assert_eq!(safety.answer(&prompt, Choice::AllowOnce), Ok(Verdict::Deny));
        safety.hand_back(1);

        safety.set_agent_paused("ion", true);
        assert_eq!(safety.answer(&prompt, Choice::Allow), Ok(Verdict::Deny));
        safety.set_agent_paused("ion", false);

        assert_eq!(safety.answer(&prompt, Choice::Allow), Ok(Verdict::Allow));
        assert_eq!(safety.request_agent(req).verdict(), Verdict::Allow);
    }

    #[test]
    fn answers_after_a_new_config_deny_deny() {
        let mut safety = Safety::in_memory();
        let req = agent_req(Action::Submit, "https://github.com");
        let prompt = safety.request_agent(req).prompt.unwrap();
        safety.set_profiles(
            [(
                "ion".to_owned(),
                AgentProfile {
                    rules: vec![crate::policy::ConfigRule {
                        what: crate::rule::What::Action(Action::Submit),
                        site: crate::site::SitePattern::Any,
                        effect: crate::rule::Effect::Deny,
                    }],
                    ..AgentProfile::default()
                },
            )]
            .into(),
        );
        assert_eq!(safety.answer(&prompt, Choice::AllowOnce), Ok(Verdict::Deny));
    }

    #[test]
    fn answers_for_closed_tabs_deny() {
        let mut safety = Safety::in_memory();
        let site = safety
            .request_site(camera("https://meet.example"))
            .prompt
            .unwrap();
        let agent = safety
            .request_agent(agent_req(Action::ReadPage, "https://github.com"))
            .prompt
            .unwrap();
        safety.tab_closed(1);
        assert_eq!(safety.answer(&site, Choice::Allow), Ok(Verdict::Deny));
        assert_eq!(safety.answer(&agent, Choice::Allow), Ok(Verdict::Deny));
        assert!(safety.policy().user_rules().is_empty());

        // Reopened with the same id: new prompts count again.
        let site = safety
            .request_site(camera("https://meet.example"))
            .prompt
            .unwrap();
        assert_eq!(safety.answer(&site, Choice::Allow), Ok(Verdict::Allow));
    }

    #[test]
    fn open_prunes_with_the_given_retention() {
        let dir = test_dir("safety-retention");
        let audit = AuditLog::new(dir.join("audit"));
        let old = system_clock() - 60 * 86_400;
        audit.append(&Entry::new(old, Kind::Stop)).unwrap();
        Safety::open(&dir, 90);
        assert_eq!(audit.read_since(0).unwrap().len(), 1);
        Safety::open(&dir, 30);
        assert!(audit.read_since(0).unwrap().is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn config_rules_cannot_be_forgotten() {
        let mut safety = Safety::in_memory();
        let rule = Rule {
            who: Who::Site,
            what: crate::rule::What::Any,
            site: crate::site::SitePattern::Any,
            effect: crate::rule::Effect::Deny,
            lifetime: Lifetime::Forever,
            source: Source::Config,
            created: 0,
        };
        assert!(!safety.forget(&rule));
    }
}
