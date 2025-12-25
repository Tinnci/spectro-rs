//! Internationalization support for spectro-gui.

use i18n_embed::{
    fluent::{fluent_language_loader, FluentLanguageLoader},
    DesktopLanguageRequester, LanguageLoader,
};
use rust_embed::RustEmbed;
use std::sync::LazyLock;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Translations;

pub static LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader = fluent_language_loader!();
    // Use "en-US" as the initial fallback
    let _ = loader.load_languages(&Translations, &[loader.fallback_language()]);
    loader
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum Language {
    #[default]
    Auto,
    EnUS,
    ZhCN,
}

impl Language {
    #[allow(dead_code)]
    pub fn to_tag(self) -> &'static str {
        match self {
            Language::Auto => "auto",
            Language::EnUS => "en-US",
            Language::ZhCN => "zh-CN",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Language::Auto => "Auto (System)",
            Language::EnUS => "English",
            Language::ZhCN => "简体中文 (Chinese)",
        }
    }
}

/// Initialize i18n with a specific language or system detection.
pub fn init(lang: Language) {
    match lang {
        Language::Auto => {
            let requested_languages = DesktopLanguageRequester::requested_languages();
            let refs: Vec<_> = requested_languages.iter().collect();
            let _ = LOADER.load_languages(&Translations, &refs);
        }
        Language::EnUS => {
            let _ = LOADER.load_languages(&Translations, &[&unic_langid::langid!("en-US")]);
        }
        Language::ZhCN => {
            let _ = LOADER.load_languages(&Translations, &[&unic_langid::langid!("zh-CN")]);
        }
    }
}

/// Translation macro for spectro-gui.
///
/// Usage:
/// ```ignore
/// use crate::i18n::t;
/// let text = t!("gui-measure");
/// ```
#[macro_export]
macro_rules! t {
    ($message_id:literal) => {
        i18n_embed_fl::fl!($crate::i18n::LOADER, $message_id)
    };
    ($message_id:literal, $($args:tt)*) => {
        i18n_embed_fl::fl!($crate::i18n::LOADER, $message_id, $($args)*)
    };
}
