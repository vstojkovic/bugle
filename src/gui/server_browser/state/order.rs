use std::cmp::Ordering;

use strum_macros::EnumIter;

use crate::gui::data::{RowComparator, RowOrder};
use crate::servers::Server;

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
}

impl RowOrder<Server> for SortCriteria {
    fn comparator(&self) -> RowComparator<Server> {
        let cmp: RowComparator<Server> = match self.key {
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
            rhs.favorite
                .cmp(&lhs.favorite)
                .then_with(|| cmp(lhs, rhs).then_with(|| tie_breaker(lhs, rhs)))
        })
    }
}
