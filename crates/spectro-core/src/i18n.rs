use i18n_embed::{
    DesktopLanguageRequester, LanguageLoader,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use rust_embed::RustEmbed;
use std::sync::LazyLock;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Translations;

pub static LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader = fluent_language_loader!();
    // 初始载入 fallback 语言
    let fallback = loader.fallback_language();
    let _ = loader.load_languages(&Translations, &[fallback]);
    loader
});

pub fn init_i18n() {
    // 自动检测并载入系统语言
    let requested_languages = DesktopLanguageRequester::requested_languages();
    let refs: Vec<_> = requested_languages.iter().collect();
    let _ = LOADER.load_languages(&Translations, &refs);
}

#[macro_export]
macro_rules! t {
    ($message_id:literal) => {
        i18n_embed_fl::fl!($crate::i18n::LOADER, $message_id)
    };
    ($message_id:literal, $($args:tt)*) => {
        i18n_embed_fl::fl!($crate::i18n::LOADER, $message_id, $($args)*)
    };
}
