use std::{fmt, marker::PhantomData};

use miette::{Result, miette};

pub struct DeclIdx<T> {
    idx: usize,
    _phantom: PhantomData<T>,
}

impl<T> DeclIdx<T> {
    /// Private method for creating a new ID.
    pub(super) fn new(idx: usize) -> Self {
        Self {
            idx,
            _phantom: PhantomData,
        }
    }

    // Convert to a `u32`, failing if it does not fit.
    pub fn try_as_u32(&self) -> Result<u32> {
        u32::try_from(self.idx).map_err(|_| miette!("index overflow: {}", self.idx))
    }
}

impl<T> Clone for DeclIdx<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for DeclIdx<T> {}

impl<T> fmt::Debug for DeclIdx<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.idx.fmt(f)
    }
}

impl<T> PartialEq for DeclIdx<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}

impl<T> Eq for DeclIdx<T> {}
