use std::cmp::Ordering;

use regex::{Regex, RegexBuilder};
use strum_macros::EnumIter;

use super::containers::Indexer;
use super::{Mode, Region, Server, ServerList};

#[derive(Clone, Copy, Debug, EnumIter, Hash, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Map,
    Mode,
    Region,
    Players,
    Age,
    Ping,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortCriteria {
    pub key: SortKey,
    pub ascending: bool,
}

macro_rules! cmp_values {
    ($ascending:expr, $($expr:tt)+) => {
        if $ascending {
            |lhs: &Server, rhs: &Server| lhs.$($expr)+.cmp(&rhs.$($expr)+)
        } else {
            |lhs: &Server, rhs: &Server| rhs.$($expr)+.cmp(&lhs.$($expr)+)
        }
    };
}

macro_rules! cmp_options {
    ($ascending:expr, $($expr:tt)+) => {
        if $ascending {
            |lhs: &Server, rhs: &Server| cmp_options!(@template lhs, rhs, lv, rv, lv, rv, $($expr)+)
        } else {
            |lhs: &Server, rhs: &Server| cmp_options!(@template lhs, rhs, lv, rv, rv, lv, $($expr)+)
        }
    };
    (@template $lhs:ident, $rhs:ident, $lbind:tt, $rbind:tt, $lval:tt, $rval:tt, $($expr:tt)+) => {
        if let Some($lbind) = $lhs.$($expr)+ {
            if let Some($rbind) = $rhs.$($expr)+ {
                $lval.cmp(&$rval)
            } else {
                Ordering::Less
            }
        } else {
            if $rhs.$($expr)+.is_some() {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }
    };
}

impl SortCriteria {
    pub fn reversed(&self) -> Self {
        Self {
            key: self.key,
            ascending: !self.ascending,
        }
    }

    fn comparator(&self) -> Box<dyn for<'s> Fn(&'s Server, &'s Server) -> Ordering> {
        let cmp: Box<dyn Fn(&Server, &Server) -> Ordering> = match self.key {
            SortKey::Name => Box::new(cmp_values!(self.ascending, name)),
            SortKey::Map => Box::new(cmp_values!(self.ascending, map)),
            SortKey::Mode => Box::new(cmp_values!(self.ascending, mode())),
            SortKey::Region => Box::new(cmp_values!(self.ascending, region)),
            SortKey::Players => Box::new({
                let connected_cmp = cmp_options!(self.ascending, connected_players);
                let max_cmp = cmp_values!(self.ascending, max_players);
                move |lhs: &Server, rhs: &Server| {
                    connected_cmp(lhs, rhs).then_with(|| max_cmp(lhs, rhs))
                }
            }),
            SortKey::Age => Box::new(cmp_options!(self.ascending, age)),
            SortKey::Ping => Box::new(cmp_options!(self.ascending, ping)),
        };
        let tie_breaker = cmp_values!(self.ascending, id);
        Box::new(move |lhs: &Server, rhs: &Server| {
            cmp(lhs, rhs).then_with(|| tie_breaker(lhs, rhs))
        })
    }
}

impl<S: ServerList> Indexer<S> for SortCriteria {
    fn index_source(&self, source: &S) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..source.len()).collect();
        let comparator = self.comparator();
        indices.sort_unstable_by(|lidx, ridx| comparator(&source[*lidx], &source[*ridx]));
        indices
    }
}

#[derive(Clone, Debug)]
pub struct Filter {
    name: String,
    name_re: Regex,
    map: String,
    map_re: Regex,
    mode: Option<Mode>,
    region: Option<Region>,
    build_id: Option<u32>,
    password_protected: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Filter::new(String::new(), String::new(), None, None, None, false)
    }
}

impl Filter {
    pub fn new(
        name: String,
        map: String,
        mode: impl Into<Option<Mode>>,
        region: impl Into<Option<Region>>,
        build_id: impl Into<Option<u32>>,
        password_protected: bool,
    ) -> Self {
        let name_re = Self::regex(&name);
        let map_re = Self::regex(&map);
        Self {
            name,
            name_re,
            map,
            map_re,
            mode: mode.into(),
            region: region.into(),
            build_id: build_id.into(),
            password_protected,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name_re = RegexBuilder::new(&regex::escape(&name))
            .case_insensitive(true)
            .build()
            .unwrap();
        self.name = name;
    }

    pub fn map(&self) -> &str {
        &self.map
    }

    pub fn set_map(&mut self, map: String) {
        self.map_re = RegexBuilder::new(&regex::escape(&map))
            .case_insensitive(true)
            .build()
            .unwrap();
        self.map = map;
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

    pub fn build_id(&self) -> Option<u32> {
        self.build_id
    }

    pub fn set_build_id(&mut self, build_id: impl Into<Option<u32>>) {
        self.build_id = build_id.into();
    }

    pub fn password_protected(&self) -> bool {
        self.password_protected
    }

    pub fn set_password_protected(&mut self, password_protected: bool) {
        self.password_protected = password_protected;
    }

    pub fn matches(&self, server: &Server) -> bool {
        self.name_re.is_match(&server.name)
            && self.map_re.is_match(&server.map)
            && self.mode.map_or(true, |mode| server.mode() == mode)
            && self.region.map_or(true, |region| server.region == region)
            && self.build_id.map_or(true, |id| server.build_id == id)
            && self.password_protected >= server.password_protected
    }

    fn regex(text: &str) -> Regex {
        RegexBuilder::new(&regex::escape(&text))
            .case_insensitive(true)
            .build()
            .unwrap()
    }
}

impl<S: ServerList> Indexer<S> for Filter {
    fn index_source(&self, source: &S) -> Vec<usize> {
        let indices: Vec<usize> = (0..source.len())
            .into_iter()
            .filter(|idx| self.matches(&source[*idx]))
            .collect();
        indices
    }
}
