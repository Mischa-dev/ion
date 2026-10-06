//! Ion browser entry point.

mod bridge;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

const MAIN_QML: &str = "qrc:/qt/qml/Ion/qml/Main.qml";

fn main() {
    // QtWebEngine has to be initialized before the application object.
    bridge::webengine::initialize_web_engine();

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

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(MAIN_QML));
    }

    let code = match app.as_mut() {
        Some(app) => app.exec(),
        None => 1,
    };
    std::process::exit(code);
}
