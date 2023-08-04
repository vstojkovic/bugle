use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{Read, Seek};

use anyhow::{anyhow, Error, Result};
use fluent_bundle::FluentResource;
use path_clean::PathClean;
use unic_langid::LanguageIdentifier;
use zip::ZipArchive;

use super::ResourceLoader;

pub struct ZipLoader<R: Read + Seek> {
    zip: ZipArchive<R>,
}

impl<R: Read + Seek> ZipLoader<R> {
    pub fn new(reader: R) -> Result<ZipLoader<R>> {
        let zip = ZipArchive::new(reader)?;
        Ok(Self { zip })
    }

    fn locales_json(&mut self) -> Result<HashMap<String, String>> {
        Ok(serde_json::from_reader(self.zip.by_name("locales.json")?)?)
    }
}

impl<R: Read + Seek> ResourceLoader for ZipLoader<R> {
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
        resources: &mut std::collections::HashMap<String, Vec<FluentResource>>,
        errors: &mut Vec<Error>,
    ) {
        let locale_str = locale.to_string();
        let locale_dir = Some(std::path::Component::Normal(OsStr::new(&locale_str)));
        let ftl_extension = Some(OsStr::new("ftl"));
        for idx in 0..self.zip.len() {
            let file = match self.zip.by_index(idx) {
                Ok(file) => file,
                Err(err) => {
                    errors.push(anyhow!(err));
                    continue;
                }
            };
            let path = match file.enclosed_name() {
                Some(path) => path.clean(),
                None => {
                    errors.push(anyhow!("Unsafe path in archive: {}", file.name()));
                    continue;
                }
            };
            if (path.components().next() != locale_dir) || (path.extension() != ftl_extension) {
                continue;
            }
            let bundle = match path.file_stem() {
                Some(stem) => stem.to_string_lossy(),
                None => continue,
            };
            let ftl_source = match std::io::read_to_string(file) {
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
