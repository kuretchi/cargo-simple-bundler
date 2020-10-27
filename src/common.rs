use crate::{file::FileInner, path::Path, Config};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
};
use string_interner::StringInterner;

pub use anyhow::Result;
pub use itertools::Itertools as _;
pub use string_interner::DefaultSymbol as Symbol;

impl fmt::Debug for WithContext<'_, '_, Symbol> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ident = self.cx.interner.resolve(*self.inner).unwrap();
        write!(f, "{}", ident)
    }
}

impl fmt::Debug for WithContext<'_, '_, HashSet<Symbol>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set()
            .entries(self.inner.iter().map(|symbol| with_context(symbol, self.cx)))
            .finish()
    }
}

pub struct Context {
    pub config: Config,
    pub interner: StringInterner,
    pub files: HashMap<Path, Rc<FileInner>>,
}

impl From<Config> for Context {
    fn from(config: Config) -> Self {
        Context { config, interner: StringInterner::new(), files: HashMap::new() }
    }
}

pub struct WithContext<'cx, 'a, T> {
    pub inner: &'a T,
    pub cx: &'cx Context,
}

pub fn with_context<'cx, 'a, T>(inner: &'a T, cx: &'cx Context) -> WithContext<'cx, 'a, T> {
    WithContext { inner, cx }
}
