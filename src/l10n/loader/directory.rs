use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Error, Result};
use fluent_bundle::FluentResource;
use unic_langid::LanguageIdentifier;

use super::ResourceLoader;

pub struct DirectoryLoader {
    base_path: PathBuf,
}

impl DirectoryLoader {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_owned(),
        }
    }

    fn locales_json(&self) -> Result<HashMap<String, String>> {
        Ok(serde_json::from_reader(File::open(
            self.base_path.join("locales.json"),
        )?)?)
    }
}

impl ResourceLoader for DirectoryLoader {
    fn enum_locales(
        &mut self,
        locales: &mut HashMap<LanguageIdentifier, String>,
        errors: &mut Vec<Error>,
    ) {
        let map = match self.locales_json() {
            Ok(map) => map,
            Err(err) => {
                errors.push(anyhow!(err));
                return;
            }
        };
        for (key, value) in map {
            let locale: LanguageIdentifier = match key.parse() {
                Ok(locale) => locale,
                Err(err) => {
                    errors.push(anyhow!(err));
                    continue;
                }
            };
            locales.insert(locale, value);
        }
    }

    fn load(
        &mut self,
        locale: &LanguageIdentifier,
        resources: &mut HashMap<String, Vec<FluentResource>>,
        errors: &mut Vec<Error>,
    ) {
        let locale_path = self.base_path.join(locale.to_string());
        if !locale_path.exists() {
            return;
        }

        let entries = match std::fs::read_dir(locale_path) {
            Ok(entries) => entries,
            Err(err) => {
                errors.push(anyhow!(err));
                return;
            }
        };

        let ftl_extension = Some(OsStr::new("ftl"));
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    errors.push(anyhow!(err));
                    continue;
                }
            };
            let entry_path = entry.path();
            let bundle = match entry_path.file_stem() {
                Some(stem) => stem.to_string_lossy(),
                None => continue,
            };
            if entry_path.extension() == ftl_extension {
                let ftl_source = match std::fs::read_to_string(&entry_path) {
                    Ok(text) => text,
                    Err(err) => {
                        errors.push(anyhow!(err));
                        continue;
                    }
                };
                let resource = match FluentResource::try_new(ftl_source) {
                    Ok(resource) => resource,
                    Err((resource, parse_errors)) => {
                        errors.extend(parse_errors.into_iter().map(|err| anyhow!(err)));
                        resource
                    }
                };
                resources
                    .entry(bundle.into())
                    .or_insert_with(|| vec![])
                    .push(resource);
            }
        }
    }
}
