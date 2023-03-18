use regex::{Regex, RegexBuilder};

use crate::config::ServerBrowserConfig;
use crate::gui::data::RowFilter;
use crate::servers::{Mode, Region, Server, TypeFilter};

#[derive(Clone, Debug)]
pub struct Filter {
    name: String,
    name_re: Regex,
    map: String,
    map_re: Regex,
    type_filter: TypeFilter,
    mode: Option<Mode>,
    region: Option<Region>,
    battleye_required: Option<bool>,
    include_invalid: bool,
    include_password_protected: bool,
    include_modded: bool,
}

impl Filter {
    pub fn from_config(config: &ServerBrowserConfig) -> Self {
        Self::new(
            String::new(),
            String::new(),
            config.type_filter,
            config.mode,
            config.region,
            config.battleye_required,
            config.include_invalid,
            config.include_password_protected,
            config.include_modded,
        )
    }

    pub fn new(
        name: String,
        map: String,
        type_filter: TypeFilter,
        mode: impl Into<Option<Mode>>,
        region: impl Into<Option<Region>>,
        battleye_required: impl Into<Option<bool>>,
        include_invalid: bool,
        include_password_protected: bool,
        include_modded: bool,
    ) -> Self {
        let name_re = Self::regex(&name);
        let map_re = Self::regex(&map);
        Self {
            name,
            name_re,
            map,
            map_re,
            type_filter,
            mode: mode.into(),
            region: region.into(),
            battleye_required: battleye_required.into(),
            include_invalid,
            include_password_protected,
            include_modded,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name_re = Self::regex(&name);
        self.name = name;
    }

    pub fn map(&self) -> &str {
        &self.map
    }

    pub fn set_map(&mut self, map: String) {
        self.map_re = Self::regex(&map);
        self.map = map;
    }

    pub fn type_filter(&self) -> TypeFilter {
        self.type_filter
    }

    pub fn set_type_filter(&mut self, type_filter: TypeFilter) {
        self.type_filter = type_filter;
    }

    pub fn mode(&self) -> Option<Mode> {
        self.mode
    }

    pub fn set_mode(&mut self, mode: impl Into<Option<Mode>>) {
        self.mode = mode.into();
    }

    pub fn region(&self) -> Option<Region> {
        self.region
    }

    pub fn set_region(&mut self, region: impl Into<Option<Region>>) {
        self.region = region.into();
    }

    pub fn battleye_required(&self) -> Option<bool> {
        self.battleye_required
    }

    pub fn set_battleye_required(&mut self, battleye_required: impl Into<Option<bool>>) {
        self.battleye_required = battleye_required.into();
    }

    pub fn include_invalid(&self) -> bool {
        self.include_invalid
    }

    pub fn set_include_invalid(&mut self, include_invalid: bool) {
        self.include_invalid = include_invalid;
    }

    pub fn include_password_protected(&self) -> bool {
        self.include_password_protected
    }

    pub fn set_include_password_protected(&mut self, include_password_protected: bool) {
        self.include_password_protected = include_password_protected;
    }

    pub fn include_modded(&self) -> bool {
        self.include_modded
    }

    pub fn set_include_modded(&mut self, include_modded: bool) {
        self.include_modded = include_modded;
    }

    fn regex(text: &str) -> Regex {
        RegexBuilder::new(&regex::escape(&text))
            .case_insensitive(true)
            .build()
            .unwrap()
    }
}

impl RowFilter<Server> for Filter {
    fn matches(&self, server: &Server) -> bool {
        self.name_re.is_match(&server.name)
            && self.map_re.is_match(&server.map)
            && self.type_filter.matches(server)
            && self.mode.map_or(true, |mode| server.mode() == mode)
            && self.region.map_or(true, |region| server.region == region)
            && self
                .battleye_required
                .map_or(true, |required| server.battleye_required == required)
            && self.include_invalid >= !server.is_valid()
            && self.include_password_protected >= server.password_protected
            && self.include_modded >= server.is_modded()
    }
}
