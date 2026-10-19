//! `Translator` QML singleton: page translation through Ion Agent's model,
//! from `ion_translate`. QML collects a page's text with `collectScript`,
//! hands it to `start`, and runs each `batchDone` script in the page as the
//! model answers. `PageTranslator.qml` drives it for one tab.

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
        #[qproperty(QString, collect_script, cxx_name = "collectScript", READ, CONSTANT)]
        #[qproperty(QString, restore_script, cxx_name = "restoreScript", READ, CONSTANT)]
        #[namespace = "ion"]
        type Translator = super::TranslatorRust;

        /// Why translation can't run now ("" when it can): Ion Agent off,
        /// or no model chosen.
        #[qinvokable]
        #[cxx_name = "unavailableReason"]
        fn unavailable_reason(self: &Translator) -> QString;

        /// Whether a page reporting `page_json` (from `collectScript`) is
        /// already in the language tagged `target`.
        #[qinvokable]
        #[cxx_name = "alreadyIn"]
        fn already_in(self: &Translator, page_json: &QString, target: &QString) -> bool;

        /// Translate the page `page_json` (from `collectScript`) into
        /// `target` (a BCP 47 tag). Returns how many batches will follow,
        /// each as `batchDone(job, …)`, then `finished(job, error)`.
        #[qinvokable]
        fn start(
            self: Pin<&mut Translator>,
            job: i32,
            page_json: &QString,
            target: &QString,
        ) -> i32;

        /// Stop a job; batches still on their way are dropped.
        #[qinvokable]
        fn cancel(self: Pin<&mut Translator>, job: i32);

        /// One batch is translated: `script` (Ion's world, main frame) puts
        /// it in the page.
        #[qsignal]
        #[cxx_name = "batchDone"]
        fn batch_done(self: Pin<&mut Translator>, job: i32, script: &QString);

        /// The job is over, with `error` "" when every batch made it.
        #[qsignal]
        fn finished(self: Pin<&mut Translator>, job: i32, error: &QString);
    }

    impl cxx_qt::Threading for Translator {}
}

use std::collections::BTreeSet;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use ion_agent::model::Message;
use ion_translate::{BATCH_CHARS, Batch};

use crate::bridge::agent::Recipe;

/// Batches one model request at a time per job; a page rarely needs many.
pub struct TranslatorRust {
    collect_script: QString,
    restore_script: QString,
    /// Jobs still wanted; a cancelled job's thread stops at the next batch.
    live: Arc<Mutex<BTreeSet<i32>>>,
}

impl Default for TranslatorRust {
    fn default() -> Self {
        Self {
            collect_script: QString::from(ion_translate::collect_script().as_str()),
            restore_script: QString::from(ion_translate::RESTORE_SCRIPT),
            live: Arc::default(),
        }
    }
}

fn recipe() -> Result<Recipe, String> {
    if !ion_config::global().config().ai.enable {
        return Err(
            "Translation uses Ion Agent's model. Turn on Ion Agent in Settings to translate pages."
                .to_owned(),
        );
    }
    Recipe::for_agent(ion_agent::invoke::DEFAULT_AGENT)
}

impl qobject::Translator {
    fn unavailable_reason(&self) -> QString {
        QString::from(recipe().err().unwrap_or_default().as_str())
    }

    fn already_in(&self, page_json: &QString, target: &QString) -> bool {
        ion_translate::parse_page(&page_json.to_string())
            .is_some_and(|(lang, _)| ion_translate::same_language(&lang, &target.to_string()))
    }

    fn start(self: Pin<&mut Self>, job: i32, page_json: &QString, target: &QString) -> i32 {
        let thread = self.qt_thread();
        let finish = move |error: String| {
            let _ = thread.queue(move |obj| obj.finished(job, &QString::from(error.as_str())));
        };
        let recipe = match recipe() {
            Ok(recipe) => recipe,
            Err(error) => {
                finish(error);
                return 0;
            }
        };
        let Some((_, texts)) = ion_translate::parse_page(&page_json.to_string()) else {
            finish("Ion couldn't read this page's text.".to_owned());
            return 0;
        };
        let batches = ion_translate::batches(&texts, BATCH_CHARS);
        if batches.is_empty() {
            finish("There's no text on this page to translate.".to_owned());
            return 0;
        }
        let count = i32::try_from(batches.len()).unwrap_or(i32::MAX);
        let live = Arc::clone(&self.rust().live);
        live.lock().unwrap_or_else(|e| e.into_inner()).insert(job);
        let target = target.to_string();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let error = run(&recipe, &batches, &target, job, &live, |script| {
                let _ =
                    thread.queue(move |obj| obj.batch_done(job, &QString::from(script.as_str())));
            });
            live.lock().unwrap_or_else(|e| e.into_inner()).remove(&job);
            let _ = thread.queue(move |obj| obj.finished(job, &QString::from(error.as_str())));
        });
        count
    }

    fn cancel(self: Pin<&mut Self>, job: i32) {
        self.rust()
            .live
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&job);
    }
}

/// Translates the batches in order, handing each one's script to `done`.
/// Returns "" or the first error; a failed batch doesn't stop the others.
fn run(
    recipe: &Recipe,
    batches: &[Batch],
    target: &str,
    job: i32,
    live: &Mutex<BTreeSet<i32>>,
    done: impl Fn(String),
) -> String {
    let model = match recipe.connect() {
        Ok(model) => model,
        Err(error) => return error,
    };
    let mut first_error = String::new();
    for batch in batches {
        if !live
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&job)
        {
            break;
        }
        let (system, user) = ion_translate::messages(batch, target);
        let reply = model
            .reply(&[Message::System(system), Message::User(user)], &[])
            .and_then(|reply| ion_translate::parse_reply(&reply.text, batch.texts.len()));
        match reply {
            Ok(translations) => done(ion_translate::apply_script(&batch.indexes, &translations)),
            Err(error) if first_error.is_empty() => first_error = error,
            Err(_) => {}
        }
    }
    first_error
}
