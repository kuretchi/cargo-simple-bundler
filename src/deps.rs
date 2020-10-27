use crate::{common::*, path::Path};
use std::{
    collections::{hash_set, HashSet},
    fmt, iter,
};
use syn::visit::Visit;

#[derive(Default)]
pub struct Deps(HashSet<Path>);

impl fmt::Debug for WithContext<'_, '_, Deps> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Deps").field(&with_context(&self.inner.0, self.cx)).finish()
    }
}

impl Deps {
    pub fn all() -> Self {
        Deps(iter::once(Path::default()).collect())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Path> {
        self.0.iter()
    }

    pub fn drain(&mut self) -> hash_set::Drain<Path> {
        self.0.drain()
    }

    pub fn insert(&mut self, path: Path) -> bool {
        self.0.insert(path)
    }
}

impl IntoIterator for Deps {
    type Item = Path;
    type IntoIter = hash_set::IntoIter<Path>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Extend<Path> for Deps {
    fn extend<I: IntoIterator<Item = Path>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}

pub fn entry_deps(entry_syn_file: &syn::File, cx: &mut Context) -> Deps {
    struct Visitor<'a> {
        deps: &'a mut Deps,
        cx: &'a mut Context,
    }

    impl<'a> Visit<'_> for Visitor<'a> {
        fn visit_item_use(&mut self, item_use: &syn::ItemUse) {
            collect_entry_deps_from_use_tree(&item_use.tree, self.deps, self.cx);
            syn::visit::visit_item_use(self, item_use);
        }
    }

    let mut deps = Deps::default();
    Visitor { deps: &mut deps, cx }.visit_file(entry_syn_file);
    deps
}

fn collect_entry_deps_from_use_tree(tree: &syn::UseTree, deps: &mut Deps, cx: &mut Context) {
    match tree {
        syn::UseTree::Path(syn::UsePath { ident, tree, .. }) if ident == &cx.config.crate_ident => {
            collect_deps_from_use_subtree(&Path::default(), tree, deps, cx);
        }
        syn::UseTree::Name(syn::UseName { ident, .. })
        | syn::UseTree::Rename(syn::UseRename { ident, .. })
            if ident == &cx.config.crate_ident =>
        {
            deps.0.insert(Path::default());
        }
        syn::UseTree::Group(syn::UseGroup { items, .. }) => {
            for tree in items {
                collect_entry_deps_from_use_tree(tree, deps, cx);
            }
        }
        _ => {}
    }
}

pub fn collect_deps_from_use_tree(
    path: &Path,
    tree: &syn::UseTree,
    deps: &mut Deps,
    cx: &mut Context,
) {
    match &tree {
        syn::UseTree::Path(syn::UsePath { ident, tree: subtree, .. }) => {
            if ident == "crate" {
                collect_deps_from_use_subtree(&Path::default(), subtree, deps, cx);
            } else if ident == "self" {
                collect_deps_from_use_subtree(path, subtree, deps, cx);
            } else if ident == "super" {
                collect_deps_from_use_subtree(path, tree, deps, cx);
            } else if ident != "std" {
                log::warn!("skip the use declaration started with `{}`", ident);
            }
        }
        syn::UseTree::Group(syn::UseGroup { items, .. }) => {
            for tree in items {
                collect_deps_from_use_tree(path, tree, deps, cx);
            }
        }
        _ => {}
    }
}

fn collect_deps_from_use_subtree(
    path: &Path,
    tree: &syn::UseTree,
    deps: &mut Deps,
    cx: &mut Context,
) {
    match tree {
        syn::UseTree::Path(syn::UsePath { ident, tree, .. }) => {
            if ident == "super" {
                if let Some(parent) = path.parent() {
                    collect_deps_from_use_subtree(&parent, tree, deps, cx);
                } else {
                    log::error!("too many `super` keywords. skip");
                }
            } else {
                let symbol = cx.interner.get_or_intern(ident.to_string());
                collect_deps_from_use_subtree(&path.child(symbol), tree, deps, cx);
            }
        }
        syn::UseTree::Name(syn::UseName { ident, .. })
        | syn::UseTree::Rename(syn::UseRename { ident, .. }) => {
            if ident == "self" {
                deps.0.insert(path.clone());
            } else {
                let symbol = cx.interner.get_or_intern(ident.to_string());
                deps.0.insert(path.child(symbol));
            }
        }
        syn::UseTree::Glob(_) => {
            deps.0.insert(path.clone());
        }
        syn::UseTree::Group(syn::UseGroup { items, .. }) => {
            for tree in items {
                collect_deps_from_use_subtree(path, tree, deps, cx);
            }
        }
    }
}
