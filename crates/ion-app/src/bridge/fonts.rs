//! Bindings to the C++ shim in `cpp/fonts.cpp`.

#[cxx_qt::bridge]
pub mod ffi {
    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/fonts.h");

        /// Load Ion's bundled fonts and pick the interface typeface. Call
        /// after the application object exists.
        #[cxx_name = "installUiFont"]
        fn install_ui_font();
    }
}

pub use ffi::install_ui_font;
