use crate::common::*;
use smallvec::SmallVec;
use std::{collections::HashSet, fmt};

#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Path(SmallVec<[Symbol; 4]>);

impl fmt::Debug for WithContext<'_, '_, Path> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "crate")?;
        for symbol in &self.inner.0 {
            write!(f, "::{:?}", with_context(symbol, self.cx))?;
        }
        Ok(())
    }
}

impl fmt::Debug for WithContext<'_, '_, HashSet<Path>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.inner.iter().map(|path| with_context(path, self.cx))).finish()
    }
}

impl Path {
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.0.iter().copied()
    }

    pub fn symbol(&self) -> Option<Symbol> {
        self.0.last().copied()
    }

    pub fn parent(&self) -> Option<Self> {
        let mut this = self.clone();
        if this.0.pop().is_some() {
            Some(this)
        } else {
            None
        }
    }

    pub fn child(&self, symbol: Symbol) -> Self {
        let mut this = self.clone();
        this.0.push(symbol);
        this
    }
}
