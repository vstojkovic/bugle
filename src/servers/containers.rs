use std::ops::{Index, IndexMut};

use super::Server;

pub trait ServerList: IndexMut<usize, Output = Server> {
    fn len(&self) -> usize;
}

impl ServerList for Vec<Server> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
}

pub trait Indexer<S: ServerList> {
    fn index_source(&self, source: &S) -> Vec<usize>;
}

impl<S: ServerList, F: Fn(&S) -> Vec<usize>> Indexer<S> for F {
    fn index_source(&self, source: &S) -> Vec<usize> {
        (self)(source)
    }
}

pub struct ServerListView<S: ServerList, I: Indexer<S>> {
    source: S,
    indexer: I,
    indices: Vec<usize>,
    inverse: Vec<Option<usize>>,
}

impl<S: ServerList, I: Indexer<S>> ServerListView<S, I> {
    pub fn new(source: S, indexer: I) -> Self {
        let indices = indexer.index_source(&source);
        let inverse = Self::map_inverse(&source, &indices);
        Self {
            source,
            indexer,
            indices,
            inverse,
        }
    }

    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn indexer(&self) -> &I {
        &self.indexer
    }

    pub fn mutate(&mut self, mutator: impl FnOnce(&mut S, &mut I) -> bool) -> bool {
        let should_reindex = mutator(&mut self.source, &mut self.indexer);
        if should_reindex {
            self.reindex();
        }
        should_reindex
    }

    pub fn reindex(&mut self) {
        self.indices = self.indexer.index_source(&self.source);
        self.inverse = Self::map_inverse(&self.source, &self.indices);
    }

    pub fn from_source_index(&self, idx: usize) -> Option<usize> {
        self.inverse[idx]
    }

    pub fn to_source_index(&self, idx: usize) -> usize {
        self.indices[idx]
    }

    fn map_inverse(source: &S, indices: &Vec<usize>) -> Vec<Option<usize>> {
        let mut inverse: Vec<Option<usize>> = vec![None; source.len()];
        for (my_idx, src_idx) in indices.iter().enumerate() {
            inverse[*src_idx] = Some(my_idx);
        }
        inverse
    }
}

impl<S: ServerList, I: Indexer<S>> Index<usize> for ServerListView<S, I> {
    type Output = Server;
    fn index(&self, index: usize) -> &Self::Output {
        &self.source[self.indices[index]]
    }
}

impl<S: ServerList, I: Indexer<S>> IndexMut<usize> for ServerListView<S, I> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.source[self.indices[index]]
    }
}

impl<S: ServerList, I: Indexer<S>> ServerList for ServerListView<S, I> {
    fn len(&self) -> usize {
        self.indices.len()
    }
}

pub struct ServerIter<'s> {
    list: &'s dyn ServerList,
    idx: usize,
}

impl<'s> ServerIter<'s> {
    fn new(list: &'s dyn ServerList) -> Self {
        Self { list, idx: 0 }
    }
}

impl<'s> Iterator for ServerIter<'s> {
    type Item = &'s Server;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.list.len() {
            return None;
        }

        let result = &self.list[self.idx];
        self.idx += 1;
        Some(result)
    }
}

impl<'s> IntoIterator for &'s dyn ServerList {
    type Item = &'s Server;
    type IntoIter = ServerIter<'s>;
    fn into_iter(self) -> Self::IntoIter {
        ServerIter::new(self)
    }
}
