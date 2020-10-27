use crate::{common::*, deps::Deps, file::file, path::Path};
use smallvec::SmallVec;
use std::{collections::HashSet, iter};

pub fn resolve_deps(mut deps: Deps, cx: &mut Context) -> Result<Deps> {
    log::debug!("resolving dependencies: {:?}", with_context(&deps, cx));

    let mut final_deps = Deps::default();
    let mut known_paths = HashSet::new();

    while !deps.is_empty() {
        log::debug!("deps = {:?}", with_context(&deps, cx));
        let mut resolved_deps = Deps::default();
        for path in deps.drain() {
            if known_paths.insert(path.clone()) {
                resolved_deps.extend(resolve_path(&path, cx)?);
            }
        }
        log::debug!("resolved_deps = {:?}", with_context(&resolved_deps, cx));
        for path in resolved_deps {
            if final_deps.insert(path.clone()) {
                let file = file(&path, cx)?;
                deps.extend(file.deps().iter().cloned());
            }
        }
    }

    Ok(final_deps)
}

fn resolve_path(path: &Path, cx: &mut Context) -> Result<SmallVec<[Path; 2]>> {
    let parent = match path.parent() {
        None => return Ok(iter::once(path.clone()).collect()), // path is root
        Some(x) => x,
    };
    log::debug!("resolving the path: {:?}", with_context(path, cx));

    let parent_file = file(&parent, cx)?;
    log::debug!("parent_file = {:?}", with_context(&parent_file, cx));

    let symbol = path.symbol().unwrap();
    let mut resolved_paths = SmallVec::new();

    if parent_file.contains_child_module(symbol) {
        log::debug!("the path resolved as a module. add: {:?}", with_context(path, cx));
        resolved_paths.push(path.clone());
    }
    if parent_file.contains_public_symbol(symbol) {
        log::debug!("the path resolved as a public item. add: {:?}", with_context(&parent, cx));
        resolved_paths.push(parent.clone());
    }
    for child_module in parent_file.child_modules() {
        if child_module.contains_reexport(symbol, cx)? {
            let path = child_module.path();
            log::debug!(
                "the path resolved as a re-exported item. add: {:?}",
                with_context(&path, cx)
            );
            resolved_paths.push(path);
        }
    }

    if resolved_paths.is_empty() {
        let ident = cx.interner.resolve(symbol).unwrap();
        log::warn!(
            "definition of `{}` not found in `{:?}`. treat as a dependency on the parent",
            ident,
            with_context(&parent, cx),
        );
        resolve_path(&parent, cx)
    } else {
        Ok(resolved_paths)
    }
}
