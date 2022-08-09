use std::cmp::Ordering;

use regex::{Regex, RegexBuilder};

use super::containers::Indexer;
use super::{Mode, Region, Server, ServerList};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Map,
    Mode,
    Region,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortCriteria {
    pub key: SortKey,
    pub ascending: bool,
}

impl SortCriteria {
    pub fn reversed(&self) -> Self {
        Self {
            key: self.key,
            ascending: !self.ascending,
        }
    }

    fn comparator(&self) -> Box<dyn FnMut(&Server, &Server) -> Ordering> {
        let cmp = match self.key {
            SortKey::Name => |lhs: &Server, rhs: &Server| lhs.name.cmp(&rhs.name),
            SortKey::Map => |lhs: &Server, rhs: &Server| lhs.map.cmp(&rhs.map),
            SortKey::Mode => |lhs: &Server, rhs: &Server| lhs.mode().cmp(&rhs.mode()),
            SortKey::Region => |lhs: &Server, rhs: &Server| lhs.region.cmp(&rhs.region),
        };
        let cmp = move |lhs: &Server, rhs: &Server| {
            cmp(lhs, rhs).then_with(|| Self::tie_breaker(lhs, rhs))
        };
        if self.ascending {
            Box::new(cmp)
        } else {
            Box::new(move |lhs, rhs| cmp(lhs, rhs).reverse())
        }
    }

    fn tie_breaker(lhs: &Server, rhs: &Server) -> Ordering {
        lhs.id.cmp(&rhs.id)
    }
}

impl<S: ServerList> Indexer<S> for SortCriteria {
    fn index_source(&self, source: &S) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..source.len()).collect();
        let mut comparator = self.comparator();
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
