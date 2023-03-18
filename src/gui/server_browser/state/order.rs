use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

use crate::gui::data::{RowComparator, RowOrder};
use crate::servers::{Region, Server, SortCriteria, SortKey};

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

pub struct SortOrder {
    pub criteria: SortCriteria,
    region_order: Rc<HashMap<Region, usize>>,
}

impl SortOrder {
    pub fn new(criteria: SortCriteria, region_order: HashMap<Region, usize>) -> Self {
        Self {
            criteria,
            region_order: Rc::new(region_order),
        }
    }

    fn region_comparator(&self) -> RowComparator<Server> {
        let region_order = Rc::clone(&self.region_order);
        if self.criteria.ascending {
            Box::new(move |lhs: &Server, rhs: &Server| {
                region_order[&lhs.region].cmp(&region_order[&rhs.region])
            })
        } else {
            Box::new(move |lhs: &Server, rhs: &Server| {
                region_order[&rhs.region].cmp(&region_order[&lhs.region])
            })
        }
    }
}

impl RowOrder<Server> for SortOrder {
    fn comparator(&self) -> RowComparator<Server> {
        let cmp: RowComparator<Server> = match self.criteria.key {
            SortKey::Name => Box::new(cmp_values!(self.criteria.ascending, name)),
            SortKey::Map => Box::new(cmp_values!(self.criteria.ascending, map)),
            SortKey::Mode => Box::new(cmp_values!(self.criteria.ascending, mode())),
            SortKey::Region => self.region_comparator(),
            SortKey::Players => Box::new({
                let connected_cmp = cmp_options!(self.criteria.ascending, connected_players);
                let max_cmp = cmp_values!(self.criteria.ascending, max_players);
                move |lhs: &Server, rhs: &Server| {
                    connected_cmp(lhs, rhs).then_with(|| max_cmp(lhs, rhs))
                }
            }),
            SortKey::Age => Box::new(cmp_options!(self.criteria.ascending, age)),
            SortKey::Ping => Box::new(cmp_options!(self.criteria.ascending, ping)),
        };
        let tie_breaker = cmp_values!(self.criteria.ascending, id);
        Box::new(move |lhs: &Server, rhs: &Server| {
            rhs.favorite
                .cmp(&lhs.favorite)
                .then_with(|| cmp(lhs, rhs).then_with(|| tie_breaker(lhs, rhs)))
        })
    }
}
