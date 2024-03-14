use std::cmp::Ordering;
use std::ops::{Index, IndexMut};

pub trait TableSource: IndexMut<usize> {
    fn len(&self) -> usize;
}

impl<T> TableSource for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
}

pub trait RowFilter<T: ?Sized> {
    fn matches(&self, item: &T) -> bool;
}

pub type RowComparator<T> = Box<dyn for<'t> Fn(&'t T, &'t T) -> Ordering>;

pub trait RowOrder<T: ?Sized> {
    fn comparator(&self) -> RowComparator<T>;
}

pub struct TableView<S: TableSource, F: RowFilter<S::Output>, O: RowOrder<S::Output>> {
    source: S,
    filter: F,
    order: O,
    indices: Vec<usize>,
    inverse: Vec<Option<usize>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Reindex {
    Filter,
    Order,
    Nothing,
}

impl Reindex {
    pub fn all() -> Self {
        Self::Filter
    }

    pub fn filter_if(self, condition: bool) -> Self {
        match self {
            Self::Filter => Self::Filter,
            other => {
                if condition {
                    Self::Filter
                } else {
                    other
                }
            }
        }
    }

    pub fn order_if(self, condition: bool) -> Self {
        match self {
            Self::Nothing => {
                if condition {
                    Self::Order
                } else {
                    Self::Nothing
                }
            }
            other => other,
        }
    }
}

impl<S: TableSource, F: RowFilter<S::Output>, O: RowOrder<S::Output>> TableView<S, F, O> {
    pub fn new(source: S, filter: F, order: O) -> Self {
        let src_len = source.len();
        let mut result = Self {
            source,
            filter,
            order,
            indices: Vec::new(),
            inverse: Vec::with_capacity(src_len),
        };
        result.reindex_filter();
        result
    }

    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn filter(&self) -> &F {
        &self.filter
    }

    pub fn order(&self) -> &O {
        &self.order
    }

    pub fn update(&mut self, mutator: impl FnOnce(&mut S, &mut F, &mut O) -> Reindex) -> Reindex {
        let should_reindex = mutator(&mut self.source, &mut self.filter, &mut self.order);
        match should_reindex {
            Reindex::Nothing => (),
            Reindex::Filter => self.reindex_filter(),
            Reindex::Order => self.reindex_order(),
        };
        should_reindex
    }

    pub fn update_source(&mut self, mutator: impl FnOnce(&mut S)) {
        self.update(|source, _, _| {
            mutator(source);
            Reindex::all()
        });
    }

    pub fn update_filter(&mut self, mutator: impl FnOnce(&mut F)) {
        self.update(|_, filter, _| {
            mutator(filter);
            Reindex::Filter
        });
    }

    pub fn update_order(&mut self, mutator: impl FnOnce(&mut O)) {
        self.update(|_, _, order| {
            mutator(order);
            Reindex::Order
        });
    }

    pub fn from_source_index(&self, idx: usize) -> Option<usize> {
        self.inverse[idx]
    }

    pub fn to_source_index(&self, idx: usize) -> usize {
        self.indices[idx]
    }

    fn reindex_filter(&mut self) {
        self.indices = (0..self.source.len())
            .into_iter()
            .filter(|&idx| self.filter.matches(&self.source[idx]))
            .collect();
        self.reindex_order();
    }

    fn reindex_order(&mut self) {
        let comparator = self.order.comparator();
        self.indices
            .sort_unstable_by(|&lidx, &ridx| comparator(&self.source[lidx], &self.source[ridx]));
        self.reindex_inverse();
    }

    fn reindex_inverse(&mut self) {
        self.inverse.clear();
        self.inverse.resize(self.source.len(), None);
        for (my_idx, &src_idx) in self.indices.iter().enumerate() {
            self.inverse[src_idx] = Some(my_idx);
        }
    }
}

impl<S, F, O> Index<usize> for TableView<S, F, O>
where
    S: TableSource,
    F: RowFilter<S::Output>,
    O: RowOrder<S::Output>,
{
    type Output = S::Output;
    fn index(&self, index: usize) -> &Self::Output {
        &self.source[self.indices[index]]
    }
}

impl<S, F, O> IndexMut<usize> for TableView<S, F, O>
where
    S: TableSource,
    F: RowFilter<S::Output>,
    O: RowOrder<S::Output>,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.source[self.indices[index]]
    }
}

impl<S, F, O> TableSource for TableView<S, F, O>
where
    S: TableSource,
    F: RowFilter<S::Output>,
    O: RowOrder<S::Output>,
{
    fn len(&self) -> usize {
        self.indices.len()
    }
}

pub struct TableIterator<'s, S: TableSource + ?Sized> {
    source: &'s S,
    idx: usize,
}

impl<'s, S: TableSource + ?Sized> Iterator for TableIterator<'s, S> {
    type Item = &'s S::Output;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.source.len() {
            return None;
        }

        let result = &self.source[self.idx];
        self.idx += 1;
        Some(result)
    }
}

pub trait IterableTableSource: TableSource {
    fn iter(&self) -> TableIterator<Self> {
        TableIterator {
            source: self,
            idx: 0,
        }
    }
}

impl<S: TableSource + ?Sized> IterableTableSource for S {}
