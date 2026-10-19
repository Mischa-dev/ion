//! Ion browser entry point.

mod bridge;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

const MAIN_QML: &str = "qrc:/qt/qml/Ion/qml/Main.qml";

fn main() {
    // Startup marks for `bench/bench.py` (printed with ION_PERF=1).
    ion_perf::startup::begin();

    // Chromium switches, and hand-off to an already running Ion.
    if !bridge::platform::startup() {
        return;
    }

    // QtWebEngine has to be initialized before the application object.
    bridge::webengine::initialize_web_engine();
    ion_perf::startup::mark("engine");

    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        app.as_mut()
            .set_application_name(&QString::from(ion_core::APP_NAME));
        app.as_mut()
            .set_application_version(&QString::from(ion_core::VERSION));
        app.as_mut()
            .set_organization_name(&QString::from(ion_core::APP_NAME));
    }
    QGuiApplication::set_desktop_file_name(&QString::from(ion_core::APP_ID));

    // Before the browser profile opens its files.
    bridge::privacy::clear_profile_files();

    // The permission and safety model, before anything can ask it.
    bridge::safety::install();
    ion_perf::startup::mark("app");

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(MAIN_QML));
    }
    ion_perf::startup::mark("window");

    let code = match app.as_mut() {
        Some(app) => app.exec(),
        None => 1,
    };
    std::process::exit(code);
}
