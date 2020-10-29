mod common;
mod concat;
mod content;
mod deps;
mod file;
mod path;
mod resolve;
mod span;

use crate::{
    common::*,
    concat::concat_contents,
    deps::{entry_deps, Deps},
    resolve::resolve_deps,
};
use anyhow::Result;
use std::{fs, io::Write, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub crate_ident: String,
    pub crate_src_dir: PathBuf,
    pub entry_file_path: Option<PathBuf>,
    pub remove_test_modules: bool,
    pub remove_doc_comments: bool,
}

pub fn bundle<W>(config: Config, writer: &mut W) -> Result<()>
where
    W: ?Sized + Write,
{
    let cx = &mut Context::from(config);
    let deps = match &cx.config.entry_file_path {
        Some(entry_file_path) => {
            let entry_content = fs::read_to_string(entry_file_path)?;
            let entry_syn_file = syn::parse_file(&entry_content)?;
            let entry_deps = entry_deps(&entry_syn_file, cx);
            log::info!("entry dependencies collected: {:?}", with_context(&entry_deps, cx));
            let deps = resolve_deps(entry_deps, cx)?;
            log::info!("dependencies resolved: {:?}", with_context(&deps, cx));
            deps
        }
        None => Deps::all(),
    };
    let content = concat_contents(&deps, cx)?;
    write!(writer, "{}", content)?;
    Ok(())
}
