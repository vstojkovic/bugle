use std::borrow::{Borrow, Cow};
use std::cell::OnceCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Mutex;

use fluent_bundle::FluentArgs;
use slog::{trace, Logger};
use unic_langid::LanguageIdentifier;

mod loader;
mod localizer;
mod macros;

use crate::logger::warn_or_crit;

pub use self::loader::{DirectoryLoader, ResourceLoader, ZipLoader};
pub use self::localizer::{Localization, Localizer};
pub(crate) use self::macros::{err, msg, use_l10n};

#[derive(Debug)]
pub struct LocalizableMessage {
    pub key: Cow<'static, str>,
    pub attr: Option<Cow<'static, str>>,
    pub args: Option<FluentArgs<'static>>,
}

#[derive(Debug)]
pub struct ErrorMessage(pub Mutex<LocalizableMessage>);

impl Display for ErrorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let localizer = localization().localizer("errors");
        let msg = self.0.lock().unwrap();
        write!(f, "{}", localizer.localize(&msg))
    }
}

impl std::error::Error for ErrorMessage {}

thread_local! {
    static LOCALIZATION: OnceCell<Rc<Localization>> = OnceCell::new();
}

pub fn init_localization(localization: Localization) {
    LOCALIZATION.with(|cell| {
        if cell.set(Rc::new(localization)).is_err() {
            panic!("Localization already initialized");
        }
    });
}

pub fn localization() -> Rc<Localization> {
    LOCALIZATION.with(|cell| Rc::clone(cell.get().unwrap()))
}

pub fn enum_locales<L: ResourceLoader>(
    logger: &Logger,
    loader: &mut L,
    strict: bool,
) -> HashMap<LanguageIdentifier, String> {
    let mut locales = HashMap::new();
    let mut errors = vec![];
    loader.enum_locales(&mut locales, &mut errors);
    if !errors.is_empty() {
        warn_or_crit!(logger, strict, "Error enumerating available locales"; "errors" => ?errors);
    }
    locales
}

pub fn select_locale(
    logger: &Logger,
    available: &HashMap<LanguageIdentifier, String>,
    preferred: impl Iterator<Item = LanguageIdentifier>,
) -> Option<LanguageIdentifier> {
    for mut locale in preferred {
        trace!(logger, "Checking if locale is supported"; "locale" => %LocaleFormatter(&locale));
        if available.contains_key(&locale) {
            return Some(locale);
        }
        locale.region = None;
        if available.contains_key(&locale) {
            return Some(locale);
        }
    }

    None
}

pub struct LocaleFormatter<L: Borrow<LanguageIdentifier>>(L);

impl<L: Borrow<LanguageIdentifier>> Display for LocaleFormatter<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.borrow().to_string())
    }
}
