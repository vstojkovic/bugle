use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use fluent_syntax::ast::Pattern;
use slog::{b, o, BorrowedKV, Logger};
use unic_langid::LanguageIdentifier;

use crate::logger::warn_or_crit;

use super::{LocalizableMessage, ResourceLoader};

pub struct Localization {
    logger: Logger,
    strict: bool,
    localizers: HashMap<String, Rc<Localizer>>,
    empty_localizer: Rc<Localizer>,
}

impl Localization {
    pub fn new<L: ResourceLoader>(
        logger: &Logger,
        locale: LanguageIdentifier,
        mut loader: L,
        strict: bool,
    ) -> Self {
        let logger = logger.new(o!("locale" => locale.to_string()));

        let mut resources = HashMap::new();
        let mut errors = vec![];

        if locale.region.is_some() {
            let mut locale = locale.clone();
            locale.region = None;
            loader.load(&locale, &mut resources, &mut errors);
        }
        loader.load(&locale, &mut resources, &mut errors);

        if !errors.is_empty() {
            warn_or_crit!(logger, strict, "Error loading localization"; "errors" => ?errors);
        }

        let mut localizers = HashMap::new();
        for (bundle, resources) in resources {
            let localizer = localizers
                .entry(bundle)
                .or_insert_with_key(|bundle| Localizer {
                    logger: logger.new(o!("bundle" => bundle.to_string())),
                    bundle: make_bundle(locale.clone()),
                    strict,
                });
            for resource in resources {
                localizer.bundle.add_resource_overriding(resource);
            }
        }
        let localizers = localizers
            .into_iter()
            .map(|(bundle, localizer)| (bundle, Rc::new(localizer)))
            .collect();

        let empty_localizer = Rc::new(Localizer {
            logger: logger.new(o!("emptyBundle" => ())),
            bundle: make_bundle(locale.clone()),
            strict,
        });

        Self {
            logger,
            strict,
            localizers,
            empty_localizer,
        }
    }

    pub fn localizer(&self, bundle: &str) -> Rc<Localizer> {
        Rc::clone(match self.localizers.get(bundle) {
            Some(localizer) => localizer,
            None => {
                warn_or_crit!(
                    self.logger, self.strict, "Missing localization bundle"; "bundle" => bundle
                );
                &self.empty_localizer
            }
        })
    }
}

pub struct Localizer {
    logger: Logger,
    bundle: FluentBundle<FluentResource>,
    strict: bool,
}

impl Localizer {
    pub fn value(&self, key: &str) -> Cow<str> {
        let pattern = match self.value_pattern(key) {
            Ok(pattern) => pattern,
            Err(err) => return err.into(),
        };
        self.translate(pattern, None, b!("key" => key))
    }

    pub fn attr(&self, key: &str, attr: &str) -> Cow<str> {
        let pattern = match self.attr_pattern(key, attr) {
            Ok(pattern) => pattern,
            Err(err) => return err.into(),
        };
        self.translate(pattern, None, b!("key" => key, "attr" => attr))
    }

    pub fn format_value(&self, key: &str, args: &FluentArgs) -> String {
        let pattern = match self.value_pattern(key) {
            Ok(pattern) => pattern,
            Err(err) => return err,
        };
        self.translate(pattern, Some(args), b!("key" => key))
            .into_owned()
    }

    pub fn format_attr(&self, key: &str, attr: &str, args: &FluentArgs) -> String {
        let pattern = match self.attr_pattern(key, attr) {
            Ok(pattern) => pattern,
            Err(err) => return err,
        };
        self.translate(pattern, Some(args), b!("key" => key, "attr" => attr))
            .into_owned()
    }

    pub fn localize(&self, msg: &LocalizableMessage) -> Cow<str> {
        let key = msg.key.as_ref();
        match (msg.attr.as_ref(), msg.args.as_ref()) {
            (None, None) => self.value(key),
            (Some(attr), None) => self.attr(key, attr.as_ref()),
            (None, Some(args)) => self.format_value(key, args).into(),
            (Some(attr), Some(args)) => self.format_attr(key, attr.as_ref(), args).into(),
        }
    }

    fn value_pattern(&self, key: &str) -> Result<&Pattern<&str>, String> {
        match self.bundle.get_message(key).and_then(|msg| msg.value()) {
            Some(pattern) => Ok(pattern),
            None => {
                warn_or_crit!(self.logger, self.strict, "Missing localization value"; "key" => key);
                Err(format!("{{?{}?}}", key))
            }
        }
    }

    fn attr_pattern(&self, key: &str, attr: &str) -> Result<&Pattern<&str>, String> {
        match self
            .bundle
            .get_message(key)
            .and_then(|msg| msg.get_attribute(attr))
        {
            Some(attr) => Ok(attr.value()),
            None => {
                warn_or_crit!(
                    self.logger,
                    self.strict,
                    "Missing localization attribute";
                    "key" => key,
                    "attr" => attr,
                );
                Err(format!("{{?{}.{}?}}", key, attr))
            }
        }
    }

    fn translate<'l, 'a: 'l>(
        &'l self,
        pattern: &'l Pattern<&str>,
        args: Option<&'a FluentArgs<'a>>,
        kv: BorrowedKV,
    ) -> Cow<'l, str> {
        let mut errors = vec![];
        let result = self.bundle.format_pattern(&pattern, args, &mut errors);
        if !errors.is_empty() {
            warn_or_crit!(
                self.logger, self.strict, "Errors in localization"; kv, "errors" => ?errors
            );
        }
        result
    }
}

fn make_bundle(locale: LanguageIdentifier) -> FluentBundle<FluentResource> {
    let mut bundle = FluentBundle::new(vec![locale]);
    bundle.set_use_isolating(false); // FLTK can't render Unicode Directionality Isolation Marks
    bundle
}
