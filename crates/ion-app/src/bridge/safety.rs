//! `Safety` QML singleton: the shell's window onto `ion_safety::global()`,
//! the permission and safety model (docs/SAFETY.md).
//!
//! QML never decides: it asks `siteDecision`, shows `sitePrompt` when the
//! answer is "ask", and hands the person's choice to `answerSite`. The stop
//! switch for agents lives here too.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, stopped, READ, NOTIFY)]
        #[namespace = "ion"]
        type Safety = super::SafetyRust;

        /// Stop every agent until `resumeAgents`.
        #[qinvokable]
        #[cxx_name = "stopAgents"]
        fn stop_agents(self: Pin<&mut Safety>);

        #[qinvokable]
        #[cxx_name = "resumeAgents"]
        fn resume_agents(self: Pin<&mut Safety>);

        /// "allow", "deny" or "ask" for a page at `origin` asking for a
        /// `WebEnginePermission.PermissionType`. Types Ion doesn't know are
        /// "deny".
        #[qinvokable]
        #[cxx_name = "siteDecision"]
        fn site_decision(self: &Safety, permission_type: i32, origin: &QUrl, tab: i32) -> QString;

        /// The prompt for that request as JSON: `{"text": "...", "choices":
        /// [{"id": "deny", "label": "Block"}, ...], "generation": 0}`,
        /// primary choice last. Empty when there is nothing to ask.
        #[qinvokable]
        #[cxx_name = "sitePrompt"]
        fn site_prompt(self: &Safety, permission_type: i32, origin: &QUrl, tab: i32) -> QString;

        /// Record the person's answer (`choice` is a choice id and
        /// `generation` the generation from the `sitePrompt` shown). Returns
        /// true if the request should be granted.
        #[qinvokable]
        #[cxx_name = "answerSite"]
        fn answer_site(
            self: &Safety,
            permission_type: i32,
            origin: &QUrl,
            tab: i32,
            generation: i32,
            choice: &QString,
        ) -> bool;

        /// Remembered site permissions as JSON, for a settings page:
        /// `[{"site": "https://meet.example", "permission": "Camera",
        /// "allowed": true}, ...]`.
        #[qinvokable]
        #[cxx_name = "sitePermissionsJson"]
        fn site_permissions_json(self: &Safety) -> QString;

        /// Forget the remembered permissions of the page at `url`. Returns
        /// how many were forgotten.
        #[qinvokable]
        #[cxx_name = "forgetSite"]
        fn forget_site(self: &Safety, url: &QUrl) -> i32;

        /// What the person chose for the page at `url`, for the toolbar's
        /// site permissions panel: `[{"type": 2, "allowed": true}, ...]`
        /// with `type` a `WebEnginePermission.PermissionType`.
        #[qinvokable]
        #[cxx_name = "sitePermissionsFor"]
        fn site_permissions_for(self: &Safety, url: &QUrl) -> QString;

        /// Remember Allow or Block for one permission of the page at `url`.
        /// Returns false for pages that can't remember (local files).
        #[qinvokable]
        #[cxx_name = "setSitePermission"]
        fn set_site_permission(
            self: &Safety,
            url: &QUrl,
            permission_type: i32,
            allowed: bool,
        ) -> bool;

        /// Forget one permission of the page at `url` so it asks again.
        #[qinvokable]
        #[cxx_name = "forgetSitePermission"]
        fn forget_site_permission(self: &Safety, url: &QUrl, permission_type: i32) -> bool;

        /// Drop tab-scoped answers and take-overs for a closed tab.
        #[qinvokable]
        #[cxx_name = "tabClosed"]
        fn tab_closed(self: &Safety, tab: i32);
    }

    impl cxx_qt::Initialize for Safety {}
}

use std::collections::BTreeMap;
use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QUrl};
use ion_safety::{
    AgentProfile, Choice, Effect, Origin, Outcome, SiteCapability, SiteRequest, TrustLevel,
    Verdict, What,
};
use serde_json::json;

#[derive(Default)]
pub struct SafetyRust {
    stopped: bool,
}

/// Open the saved model and keep it in step with config. Call once from
/// `main` before QML loads.
pub fn install() {
    let retention = ion_config::global().config().safety.audit_retention_days;
    let safety = match ion_session::paths::data_dir() {
        Some(dir) => ion_safety::Safety::open(&dir.join("safety"), retention),
        None => ion_safety::Safety::in_memory(),
    };
    ion_safety::install_global(safety);
    apply_config(&ion_config::global().config());
    let subscription = ion_config::global().subscribe(|state| apply_config(&state.config));
    // Lives as long as the process, like the store it watches.
    std::mem::forget(subscription);
}

fn apply_config(config: &ion_config::Config) {
    let mut profiles = BTreeMap::new();
    for (id, agent) in &config.agents.profiles {
        let rules: Vec<(String, String, Effect)> = agent
            .rules
            .iter()
            .map(|r| {
                let effect = match r.effect {
                    ion_config::RuleEffect::Allow => Effect::Allow,
                    ion_config::RuleEffect::Ask => Effect::Ask,
                    ion_config::RuleEffect::Deny => Effect::Deny,
                };
                (r.action.clone(), r.site.clone(), effect)
            })
            .collect();
        let trust = TrustLevel::from_id(agent.trust.as_str()).unwrap_or_default();
        let (profile, warnings) = AgentProfile::from_config(
            id,
            &agent.name,
            trust,
            &agent.trusted_sites,
            &agent.connectors,
            &rules,
        );
        for warning in warnings {
            eprintln!("ion: config: {warning}");
        }
        profiles.insert(id.clone(), profile);
    }
    let mut safety = ion_safety::global();
    safety.set_profiles(profiles);
    safety.set_audit_retention_days(config.safety.audit_retention_days);
}

fn site_request(permission_type: i32, origin: &QUrl, tab: i32) -> Option<SiteRequest> {
    Some(SiteRequest {
        capability: SiteCapability::from_qt(permission_type)?,
        origin: Origin::parse(&origin.to_string())?,
        tab: u64::try_from(tab).ok(),
    })
}

fn verdict_word(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Allow => "allow",
        Verdict::Ask => "ask",
        Verdict::Deny => "deny",
    }
}

impl cxx_qt::Initialize for qobject::Safety {
    fn initialize(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().stopped = ion_safety::global().is_stopped();
    }
}

impl qobject::Safety {
    fn set_stopped_state(mut self: Pin<&mut Self>, stopped: bool) {
        if self.stopped != stopped {
            self.as_mut().rust_mut().stopped = stopped;
            self.as_mut().stopped_changed();
        }
    }

    fn stop_agents(self: Pin<&mut Self>) {
        ion_safety::global().stop_all();
        self.set_stopped_state(true);
    }

    fn resume_agents(self: Pin<&mut Self>) {
        ion_safety::global().resume_all();
        self.set_stopped_state(false);
    }

    fn outcome(permission_type: i32, origin: &QUrl, tab: i32) -> Option<Outcome> {
        site_request(permission_type, origin, tab).map(|r| ion_safety::global().request_site(r))
    }

    fn site_decision(&self, permission_type: i32, origin: &QUrl, tab: i32) -> QString {
        let verdict =
            Self::outcome(permission_type, origin, tab).map_or(Verdict::Deny, |o| o.verdict());
        QString::from(verdict_word(verdict))
    }

    fn site_prompt(&self, permission_type: i32, origin: &QUrl, tab: i32) -> QString {
        let Some(prompt) = Self::outcome(permission_type, origin, tab).and_then(|o| o.prompt)
        else {
            return QString::default();
        };
        let choices: Vec<_> = prompt
            .choices
            .iter()
            .map(|(choice, label)| json!({ "id": choice.id(), "label": label }))
            .collect();
        QString::from(
            json!({
                "text": prompt.text,
                "choices": choices,
                "generation": prompt.tab_generation,
            })
            .to_string()
            .as_str(),
        )
    }

    fn answer_site(
        &self,
        permission_type: i32,
        origin: &QUrl,
        tab: i32,
        generation: i32,
        choice: &QString,
    ) -> bool {
        let Ok(generation) = u32::try_from(generation) else {
            return false;
        };
        let Some(choice) = Choice::from_id(&choice.to_string()) else {
            return false;
        };
        let Some(request) = site_request(permission_type, origin, tab) else {
            return false;
        };
        ion_safety::global().answer_site(request, generation, choice) == Ok(Verdict::Allow)
    }

    fn site_permissions_json(&self) -> QString {
        let safety = ion_safety::global();
        let list: Vec<_> = safety
            .site_permissions()
            .into_iter()
            .filter_map(|rule| match rule.what {
                What::Site(capability) => Some(json!({
                    "site": rule.site.to_string(),
                    "permission": capability.label(),
                    "allowed": rule.effect == Effect::Allow,
                })),
                _ => None,
            })
            .collect();
        QString::from(serde_json::Value::from(list).to_string().as_str())
    }

    fn forget_site(&self, url: &QUrl) -> i32 {
        Origin::parse(&url.to_string())
            .map_or(0, |o| ion_safety::global().forget_site(&o))
            .try_into()
            .unwrap_or(i32::MAX)
    }

    fn site_permissions_for(&self, url: &QUrl) -> QString {
        let list: Vec<_> = Origin::parse(&url.to_string())
            .map(|origin| ion_safety::global().site_permissions_for(&origin))
            .unwrap_or_default()
            .into_iter()
            .map(|(capability, effect)| {
                json!({ "type": capability.to_qt(), "allowed": effect == Effect::Allow })
            })
            .collect();
        QString::from(serde_json::Value::from(list).to_string().as_str())
    }

    fn set_site_permission(&self, url: &QUrl, permission_type: i32, allowed: bool) -> bool {
        let (Some(origin), Some(capability)) = (
            Origin::parse(&url.to_string()),
            SiteCapability::from_qt(permission_type),
        ) else {
            return false;
        };
        ion_safety::global().set_site_permission(&origin, capability, allowed)
    }

    fn forget_site_permission(&self, url: &QUrl, permission_type: i32) -> bool {
        let (Some(origin), Some(capability)) = (
            Origin::parse(&url.to_string()),
            SiteCapability::from_qt(permission_type),
        ) else {
            return false;
        };
        ion_safety::global().forget_site_permission(&origin, capability)
    }

    fn tab_closed(&self, tab: i32) {
        if let Ok(tab) = u64::try_from(tab) {
            ion_safety::global().tab_closed(tab);
        }
    }
}
