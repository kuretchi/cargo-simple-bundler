use crate::{
    common::*,
    deps::{collect_deps_from_use_tree, Deps},
    path::Path,
    span::Span,
};
use std::{
    collections::{HashMap, HashSet},
    fmt, fs,
    path::PathBuf as FsPathBuf,
    rc::Rc,
};
use syn::{
    spanned::Spanned as _,
    visit::{self, Visit},
};

pub struct File<'a> {
    path: &'a Path,
    inner: Rc<FileInner>,
}

impl fmt::Debug for WithContext<'_, '_, File<'_>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File")
            .field("path", &with_context(self.inner.path, self.cx))
            .field("inner", &with_context(&*self.inner.inner, self.cx))
            .finish()
    }
}

impl File<'_> {
    pub fn child_modules(&self) -> impl Iterator<Item = ChildModule<'_>> {
        let parent_path = self.path;
        self.inner.child_modules.iter().map(move |(&symbol, inner)| ChildModule {
            parent_path,
            symbol,
            inner,
        })
    }

    pub fn content(&self) -> &str {
        &self.inner.content
    }

    pub fn deps(&self) -> &Deps {
        &self.inner.deps
    }

    pub fn contains_public_symbol(&self, symbol: Symbol) -> bool {
        self.inner.public_symbols.contains(&symbol)
    }

    pub fn contains_child_module(&self, symbol: Symbol) -> bool {
        self.inner.child_modules.contains_key(&symbol)
    }

    pub fn crate_keyword_spans(&self) -> impl Iterator<Item = Span> + '_ {
        self.inner.crate_keyword_spans.iter().copied()
    }

    pub fn doc_comment_spans(&self) -> impl Iterator<Item = Span> + '_ {
        self.inner.doc_comment_spans.iter().copied()
    }

    pub fn test_module_spans(&self) -> impl Iterator<Item = Span> + '_ {
        self.inner.test_module_spans.iter().copied()
    }
}

#[derive(Default)]
pub struct FileInner {
    content: String,
    deps: Deps,
    public_symbols: HashSet<Symbol>,
    child_modules: HashMap<Symbol, ChildModuleInner>,
    crate_keyword_spans: Vec<Span>,
    doc_comment_spans: Vec<Span>,
    test_module_spans: Vec<Span>,
}

impl fmt::Debug for WithContext<'_, '_, FileInner> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileInner")
            .field("deps", &with_context(&self.inner.deps, self.cx))
            .field("public_symbols", &with_context(&self.inner.public_symbols, self.cx))
            .field("child_modules", &with_context(&self.inner.child_modules, self.cx))
            .finish()
    }
}

impl FileInner {
    fn parse(content: String, syn_file: &syn::File, path: &Path, cx: &mut Context) -> FileInner {
        log::debug!("analyzing the file: {:?}", with_context(path, cx));

        let public_symbols = public_symbols(syn_file, cx);
        let mut child_modules = child_modules(syn_file, cx);
        collect_reexports(syn_file, &mut child_modules, cx);

        let mut file = FileInner { content, public_symbols, child_modules, ..FileInner::default() };
        Visitor1 { file: &mut file, path, cx }.visit_file(syn_file);
        Visitor2 { file: &mut file }.visit_file(syn_file);

        log::debug!("the file analyzed: {:?}", with_context(&file, cx));

        file
    }
}

fn public_symbols(syn_file: &syn::File, cx: &mut Context) -> HashSet<Symbol> {
    syn_file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Const(syn::ItemConst { vis, ident, .. })
            | syn::Item::Enum(syn::ItemEnum { vis, ident, .. })
            | syn::Item::Fn(syn::ItemFn { vis, sig: syn::Signature { ident, .. }, .. })
            | syn::Item::Macro2(syn::ItemMacro2 { vis, ident, .. })
            | syn::Item::Static(syn::ItemStatic { vis, ident, .. })
            | syn::Item::Struct(syn::ItemStruct { vis, ident, .. })
            | syn::Item::Trait(syn::ItemTrait { vis, ident, .. })
            | syn::Item::TraitAlias(syn::ItemTraitAlias { vis, ident, .. })
            | syn::Item::Type(syn::ItemType { vis, ident, .. })
            | syn::Item::Union(syn::ItemUnion { vis, ident, .. })
                if matches!(vis, syn::Visibility::Public(_)) =>
            {
                let symbol = cx.interner.get_or_intern(ident.to_string());
                Some(symbol)
            }
            _ => None,
        })
        .collect()
}

fn child_modules(syn_file: &syn::File, cx: &mut Context) -> HashMap<Symbol, ChildModuleInner> {
    syn_file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Mod(item_mod) if item_mod.content.is_none() => {
                let symbol = cx.interner.get_or_intern(item_mod.ident.to_string());
                let child_module = ChildModuleInner {
                    item_mod_span: item_mod.span().into(),
                    item_mod_semi_span: item_mod.semi.unwrap().span().into(),
                    ..ChildModuleInner::default()
                };
                Some((symbol, child_module))
            }
            _ => None,
        })
        .collect()
}

fn collect_reexports(
    syn_file: &syn::File,
    child_modules: &mut HashMap<Symbol, ChildModuleInner>,
    cx: &mut Context,
) {
    for item_use in syn_file.items.iter().filter_map(|item| match item {
        syn::Item::Use(item_use) => Some(item_use),
        _ => None,
    }) {
        do_collect_reexports(item_use, child_modules, cx);
    }
}

fn do_collect_reexports(
    item_use: &syn::ItemUse,
    child_modules: &mut HashMap<Symbol, ChildModuleInner>,
    cx: &mut Context,
) {
    let syn::ItemUse { vis, tree, .. } = item_use;
    if !matches!(vis, syn::Visibility::Public(_)) {
        return;
    }
    let (child_module, tree) = {
        let (ident, tree) = match tree {
            syn::UseTree::Path(syn::UsePath { ident, tree, .. }) => {
                if ident == "self" {
                    match &**tree {
                        syn::UseTree::Path(syn::UsePath { ident, tree, .. }) => (ident, tree),
                        _ => return,
                    }
                } else {
                    (ident, tree)
                }
            }
            _ => return,
        };
        let symbol = cx.interner.get_or_intern(ident.to_string());
        match child_modules.get_mut(&symbol) {
            Some(child_module) => (child_module, tree),
            None => return,
        }
    };
    match &**tree {
        syn::UseTree::Name(syn::UseName { ident, .. })
        | syn::UseTree::Rename(syn::UseRename { rename: ident, .. }) => {
            let symbol = cx.interner.get_or_intern(ident.to_string());
            child_module.reexports.insert(symbol);
        }
        syn::UseTree::Glob(_) => {
            child_module.reexports = Reexports::Glob;
        }
        // TODO: support more nested groups
        syn::UseTree::Group(syn::UseGroup { items, .. }) => {
            for tree in items {
                match tree {
                    syn::UseTree::Name(syn::UseName { ident, .. })
                    | syn::UseTree::Rename(syn::UseRename { rename: ident, .. }) => {
                        let symbol = cx.interner.get_or_intern(ident.to_string());
                        child_module.reexports.insert(symbol);
                    }
                    _ => {}
                }
            }
        }
        _ => return,
    };
    child_module.item_use_span = Some(item_use.span().into());
}

struct Visitor1<'a> {
    file: &'a mut FileInner,
    path: &'a Path,
    cx: &'a mut Context,
}

impl<'a> Visit<'_> for Visitor1<'a> {
    fn visit_attribute(&mut self, attr: &syn::Attribute) {
        if self.cx.config.remove_doc_comments && attr.path.is_ident("doc") {
            self.file.doc_comment_spans.push(attr.span().into());
        }
        visit::visit_attribute(self, attr);
    }

    fn visit_item_use(&mut self, item_use: &syn::ItemUse) {
        match item_use.vis {
            syn::Visibility::Crate(_) | syn::Visibility::Restricted(_) => {
                log::warn!("skip a use declaration with `pub(restricted)`");
            }
            syn::Visibility::Inherited => {
                collect_deps_from_use_tree(self.path, &item_use.tree, &mut self.file.deps, self.cx);
            }
            _ => {}
        }
        visit::visit_item_use(self, item_use);
    }

    fn visit_item_mod(&mut self, item_mod: &syn::ItemMod) {
        if item_mod.content.is_some() {
            if self.cx.config.remove_test_modules
                && item_mod.attrs.iter().any(is_cfg_test_attribute)
            {
                self.file.test_module_spans.push(item_mod.span().into());
            } else {
                log::warn!("skip the inline module `{}`", item_mod.ident);
            }
            return;
        }
        visit::visit_item_mod(self, item_mod);
    }
}

fn is_cfg_test_attribute(attr: &syn::Attribute) -> bool {
    thread_local! {
        static CFG_TEST_ATTR: syn::Attribute = {
            use syn::parse::Parser as _;
            let attrs = syn::Attribute::parse_outer.parse_str("#[cfg(test)]").unwrap();
            attrs.into_iter().exactly_one().unwrap()
        };
    }
    CFG_TEST_ATTR.with(|x| attr == x)
}

struct Visitor2<'a> {
    file: &'a mut FileInner,
}

impl Visit<'_> for Visitor2<'_> {
    fn visit_item_use(&mut self, item_use: &syn::ItemUse) {
        collect_crate_keyword_spans(&item_use.tree, &mut self.file.crate_keyword_spans);
        visit::visit_item_use(self, item_use);
    }

    fn visit_item_macro(&mut self, item_macro: &syn::ItemMacro) {
        collect_dollar_crate_keyword_spans(
            item_macro.mac.tokens.clone(),
            &mut self.file.crate_keyword_spans,
        );
        visit::visit_item_macro(self, item_macro);
    }
}

fn collect_crate_keyword_spans(tree: &syn::UseTree, spans: &mut Vec<Span>) {
    match tree {
        syn::UseTree::Path(syn::UsePath { ident, .. }) if ident == "crate" => {
            spans.push(ident.span().into());
        }
        syn::UseTree::Group(syn::UseGroup { items, .. }) => {
            for tree in items {
                collect_crate_keyword_spans(tree, spans);
            }
        }
        _ => {}
    }
}

fn collect_dollar_crate_keyword_spans(tokens: proc_macro2::TokenStream, spans: &mut Vec<Span>) {
    let iter = tokens.clone().into_iter().tuple_windows().filter_map(|(token0, token1)| {
        match (token0, token1) {
            (proc_macro2::TokenTree::Punct(punct), proc_macro2::TokenTree::Ident(ident))
                if punct.as_char() == '$' && ident == "crate" =>
            {
                Some(Span::from(ident.span()))
            }
            _ => None,
        }
    });
    spans.extend(iter);

    for group in tokens.into_iter().filter_map(|token| match token {
        proc_macro2::TokenTree::Group(group) => Some(group),
        _ => None,
    }) {
        collect_dollar_crate_keyword_spans(group.stream(), spans);
    }
}

pub fn file<'a>(path: &'a Path, cx: &mut Context) -> Result<File<'a>> {
    if let Some(file) = cx.files.get(path) {
        return Ok(File { path, inner: Rc::clone(file) });
    }
    let fs_path = fs_path(path, cx);
    let content = fs::read_to_string(&fs_path)?;
    let syn_file = syn::parse_file(&content)?;
    let file = Rc::new(FileInner::parse(content, &syn_file, path, cx));
    cx.files.insert(path.clone(), Rc::clone(&file));
    Ok(File { path, inner: file })
}

fn fs_path(path: &Path, cx: &mut Context) -> FsPathBuf {
    let mut fs_path = cx.config.crate_src_dir.to_owned();
    if path.is_root() {
        fs_path.push("lib");
    } else {
        fs_path.extend(path.iter().map(|symbol| cx.interner.resolve(symbol).unwrap()));
    }
    fs_path.with_extension("rs")
}

pub struct ChildModule<'a> {
    parent_path: &'a Path,
    symbol: Symbol,
    inner: &'a ChildModuleInner,
}

impl ChildModule<'_> {
    pub fn symbol(&self) -> Symbol {
        self.symbol
    }

    pub fn path(&self) -> Path {
        self.parent_path.child(self.symbol)
    }

    pub fn contains_reexport(&self, symbol: Symbol, cx: &mut Context) -> Result<bool> {
        Ok(match &self.inner.reexports {
            Reexports::Glob => {
                let path = self.path();
                let child_file = file(&path, cx)?;
                child_file.contains_public_symbol(symbol)
            }
            Reexports::Group(set) => set.contains(&symbol),
        })
    }

    pub fn item_mod_span(&self) -> Span {
        self.inner.item_mod_span
    }

    pub fn item_mod_semi_span(&self) -> Span {
        self.inner.item_mod_semi_span
    }

    pub fn item_use_span(&self) -> Option<Span> {
        self.inner.item_use_span
    }
}

#[derive(Default)]
pub struct ChildModuleInner {
    reexports: Reexports,
    item_mod_span: Span,
    item_mod_semi_span: Span,
    item_use_span: Option<Span>,
}

impl fmt::Debug for WithContext<'_, '_, ChildModuleInner> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChildModuleInner")
            .field("reexports", &with_context(&self.inner.reexports, self.cx))
            .finish()
    }
}

impl fmt::Debug for WithContext<'_, '_, HashMap<Symbol, ChildModuleInner>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.inner.iter().map(|(symbol, child_module)| {
                (with_context(symbol, self.cx), with_context(child_module, self.cx))
            }))
            .finish()
    }
}

pub enum Reexports {
    Glob,
    Group(HashSet<Symbol>),
}

impl Default for Reexports {
    fn default() -> Self {
        Reexports::Group(HashSet::default())
    }
}

impl fmt::Debug for WithContext<'_, '_, Reexports> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner {
            Reexports::Glob => write!(f, "*"),
            Reexports::Group(set) => write!(f, "{:?}", with_context(set, self.cx)),
        }
    }
}

impl Reexports {
    fn insert(&mut self, symbol: Symbol) {
        match self {
            Reexports::Glob => {}
            Reexports::Group(set) => {
                set.insert(symbol);
            }
        }
    }
}
