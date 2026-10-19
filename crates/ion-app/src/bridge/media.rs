//! `Media` QML singleton: autoplay policy, from `ion_media`.

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
        #[qproperty(
            QString,
            block_autoplay_script,
            cxx_name = "blockAutoplayScript",
            READ,
            CONSTANT
        )]
        #[qproperty(
            QString,
            unblock_autoplay_script,
            cxx_name = "unblockAutoplayScript",
            READ,
            CONSTANT
        )]
        #[namespace = "ion"]
        type Media = super::MediaRust;

        /// What pages at `url` may play on their own: `"allow"`, `"muted"`
        /// or `"block"` (`media.autoplay`, after `[sites]` overrides).
        #[qinvokable]
        fn autoplay(self: &Media, url: &QString) -> QString;
    }
}

use cxx_qt_lib::QString;

pub struct MediaRust {
    block_autoplay_script: QString,
    unblock_autoplay_script: QString,
}

impl Default for MediaRust {
    fn default() -> Self {
        Self {
            block_autoplay_script: QString::from(ion_media::autoplay::BLOCK_SCRIPT),
            unblock_autoplay_script: QString::from(ion_media::autoplay::UNBLOCK_SCRIPT),
        }
    }
}

impl qobject::Media {
    fn autoplay(&self, url: &QString) -> QString {
        let config = ion_config::global().config();
        let policy =
            ion_media::autoplay_for(config.media.autoplay, &config.sites, &url.to_string());
        QString::from(policy.as_str())
    }
}
