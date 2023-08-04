use std::collections::HashMap;

use anyhow::Error;
use fluent_bundle::FluentResource;
use unic_langid::LanguageIdentifier;

mod directory;
mod zip;

pub use self::directory::DirectoryLoader;
pub use self::zip::ZipLoader;

pub trait ResourceLoader {
    fn enum_locales(
        &mut self,
        locales: &mut HashMap<LanguageIdentifier, String>,
        errors: &mut Vec<Error>,
    );
    fn load(
        &mut self,
        locale: &LanguageIdentifier,
        resources: &mut HashMap<String, Vec<FluentResource>>,
        errors: &mut Vec<Error>,
    );
}

impl<Loader: ResourceLoader, Fallback: ResourceLoader> ResourceLoader for (Loader, Fallback) {
    fn enum_locales(
        &mut self,
        locales: &mut HashMap<LanguageIdentifier, String>,
        errors: &mut Vec<Error>,
    ) {
        self.1.enum_locales(locales, errors);
        self.0.enum_locales(locales, errors);
    }

    fn load(
        &mut self,
        locale: &LanguageIdentifier,
        resources: &mut HashMap<String, Vec<FluentResource>>,
        errors: &mut Vec<Error>,
    ) {
        self.1.load(locale, resources, errors);
        self.0.load(locale, resources, errors);
    }
}
