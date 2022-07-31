use std::cmp::Ordering;
use std::ops::{Deref, Index};
use std::sync::Arc;

use super::Server;

pub trait Servers: Index<usize, Output = Server> + Send + Sync {
    fn len(&self) -> usize;
}

impl Servers for Vec<Server> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
}

#[derive(Clone)]
pub struct ServerList {
    servers: Arc<dyn Servers>,
}

impl Deref for ServerList {
    type Target = dyn Servers;
    fn deref(&self) -> &Self::Target {
        self.servers.deref()
    }
}

impl<S: Servers + 'static> From<S> for ServerList {
    fn from(servers: S) -> Self {
        Self {
            servers: Arc::new(servers),
        }
    }
}

impl ServerList {
    pub fn empty() -> Self {
        Self {
            servers: Arc::new(vec![]),
        }
    }

    pub fn sorted(&self, criteria: SortCriteria) -> Self {
        Self {
            servers: Arc::new(ServerListView::sorted_from(self.servers.clone(), criteria)),
        }
    }
}

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
            SortKey::Name => |lhs: &Server, rhs: &Server| {
                let lname = if let Some(name) = lhs.name.as_ref() { name } else { "" };
                let rname = if let Some(name) = rhs.name.as_ref() { name } else { "" };
                lname.cmp(rname)
            },
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

struct ServerListView {
    source: Arc<dyn Servers>,
    indices: Vec<usize>,
}

impl ServerListView {
    fn sorted_from(source: Arc<dyn Servers>, criteria: SortCriteria) -> Self {
        let mut indices: Vec<usize> = (0..source.len()).collect();
        let mut comparator = criteria.comparator();
        indices.sort_unstable_by(|lidx, ridx| comparator(&source[*lidx], &source[*ridx]));
        Self { source, indices }
    }
}

impl Index<usize> for ServerListView {
    type Output = Server;
    fn index(&self, index: usize) -> &Self::Output {
        &self.source[self.indices[index]]
    }
}

impl Servers for ServerListView {
    fn len(&self) -> usize {
        self.indices.len()
    }
}

impl<'l> IntoIterator for &'l dyn Servers {
    type Item = &'l Server;
    type IntoIter = ServerIter<'l>;

    fn into_iter(self) -> Self::IntoIter {
        ServerIter::new(self)
    }
}

pub struct ServerIter<'l> {
    list: &'l dyn Servers,
    idx: usize,
}

impl<'l> ServerIter<'l> {
    fn new(list: &'l dyn Servers) -> Self {
        Self { list, idx: 0 }
    }
}

impl<'l> Iterator for ServerIter<'l> {
    type Item = &'l Server;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.list.len() {
            return None;
        }

        let result = &self.list[self.idx];
        self.idx += 1;
        Some(result)
    }
}
