use regex::{Regex, RegexBuilder};

use crate::config::ServerBrowserConfig;
use crate::gui::data::RowFilter;
use crate::servers::{Mode, Region, Server, TypeFilter};

#[derive(Clone, Debug)]
pub struct Filter {
    values: crate::servers::Filter,
    name_re: Regex,
    map_re: Regex,
}

impl Filter {
    pub fn from_config(config: &ServerBrowserConfig) -> Self {
        Self {
            values: config.filter.clone(),
            name_re: Self::regex(&config.filter.name),
            map_re: Self::regex(&config.filter.map),
        }
    }

    pub fn name(&self) -> &str {
        &self.values.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name_re = Self::regex(&name);
        self.values.name = name;
    }

    pub fn map(&self) -> &str {
        &self.values.map
    }

    pub fn set_map(&mut self, map: String) {
        self.map_re = Self::regex(&map);
        self.values.map = map;
    }

    pub fn type_filter(&self) -> TypeFilter {
        self.values.type_filter
    }

    pub fn set_type_filter(&mut self, type_filter: TypeFilter) {
        self.values.type_filter = type_filter;
    }

    pub fn mode(&self) -> Option<Mode> {
        self.values.mode
    }

    pub fn set_mode(&mut self, mode: impl Into<Option<Mode>>) {
        self.values.mode = mode.into();
    }

    pub fn region(&self) -> Option<Region> {
        self.values.region
    }

    pub fn set_region(&mut self, region: impl Into<Option<Region>>) {
        self.values.region = region.into();
    }

    pub fn battleye_required(&self) -> Option<bool> {
        self.values.battleye_required
    }

    pub fn set_battleye_required(&mut self, battleye_required: impl Into<Option<bool>>) {
        self.values.battleye_required = battleye_required.into();
    }

    pub fn include_invalid(&self) -> bool {
        self.values.include_invalid
    }

    pub fn set_include_invalid(&mut self, include_invalid: bool) {
        self.values.include_invalid = include_invalid;
    }

    pub fn include_password_protected(&self) -> bool {
        !self.values.exclude_password_protected
    }

    pub fn set_include_password_protected(&mut self, include_password_protected: bool) {
        self.values.exclude_password_protected = !include_password_protected;
    }

    pub fn mods(&self) -> Option<bool> {
        self.values.mods
    }

    pub fn set_mods(&mut self, mods: impl Into<Option<bool>>) {
        self.values.mods = mods.into();
    }

    fn regex(text: &str) -> Regex {
        RegexBuilder::new(&regex::escape(&text))
            .case_insensitive(true)
            .build()
            .unwrap()
    }
}

impl AsRef<crate::servers::Filter> for Filter {
    fn as_ref(&self) -> &crate::servers::Filter {
        &self.values
    }
}

impl RowFilter<Server> for Filter {
    fn matches(&self, server: &Server) -> bool {
        !server.tombstone
            && self.name_re.is_match(&server.name)
            && self.map_re.is_match(&server.map)
            && self.values.type_filter.matches(server)
            && self.values.mode.map_or(true, |mode| server.mode() == mode)
            && self
                .values
                .region
                .map_or(true, |region| server.region == region)
            && self.values.battleye_required.map_or(true, |required| {
                server.general.battleye_required == required
            })
            && self.values.include_invalid >= !server.is_valid()
            && !(self.values.exclude_password_protected && server.password_protected)
            && self
                .values
                .mods
                .map_or(true, |mods| server.is_modded() == mods)
    }
}
