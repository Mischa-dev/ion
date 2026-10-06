//! Bindings to the C++ shims in `cpp/webengine.cpp`.

#[cxx_qt::bridge]
pub mod ffi {
    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/webengine.h");

        /// Set up QtWebEngine. Must be called before the application object exists.
        #[cxx_name = "initializeWebEngine"]
        fn initialize_web_engine();
    }
}

pub use ffi::initialize_web_engine;
