use cargo_simple_bundler::{bundle, Config};

use anyhow::{anyhow, Result};
use itertools::Itertools as _;
use std::{
    io::{self, prelude::*, BufWriter},
    path::PathBuf,
};
use structopt::{clap, StructOpt};

#[derive(StructOpt)]
#[structopt(
    bin_name("cargo"),
    global_settings(&[clap::AppSettings::UnifiedHelpMessage])
)]
enum Opt {
    SimpleBundler {
        #[structopt(long, value_name = "PATH", help = "Path to Cargo.toml")]
        manifest_path: Option<PathBuf>,
        #[structopt(long, help = "Remove inline modules with `#[cfg(test)]`")]
        remove_test_modules: bool,
        #[structopt(long, help = "Remove doc comments")]
        remove_doc_comments: bool,
        #[structopt(
            short = "e",
            long,
            value_name = "PATH",
            help = "Specify the path to the entry file to enable dependency analysis"
        )]
        entry_file_path: Option<PathBuf>,
        #[structopt(
            long,
            name = "NUM",
            help = "Enable indentation with the specified number of spaces"
        )]
        indent_spaces: Option<usize>,
    },
}

fn main() -> Result<()> {
    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "warn");
    env_logger::Builder::from_env(env).format_module_path(false).format_timestamp(None).init();

    let Opt::SimpleBundler {
        manifest_path,
        remove_test_modules,
        remove_doc_comments,
        entry_file_path,
        indent_spaces,
    } = Opt::from_args();

    let config = {
        let metadata = {
            let mut cmd = cargo_metadata::MetadataCommand::new();
            if let Some(path) = manifest_path {
                cmd.manifest_path(path);
            }
            cmd.exec()?
        };
        let package = metadata.root_package().ok_or_else(|| anyhow!("root package not found"))?;
        let target = package
            .targets
            .iter()
            .filter(|target| {
                target.name == package.name && target.kind.iter().any(|kind| kind.ends_with("lib"))
            })
            .exactly_one()
            .map_err(|_| anyhow!("target not found or multiple targets found"))?;

        Config {
            crate_ident: package.name.replace('-', "_"),
            crate_src_dir: target.src_path.parent().unwrap().to_owned(),
            entry_file_path,
            remove_test_modules,
            remove_doc_comments,
            indent_spaces: indent_spaces.unwrap_or(0),
        }
    };

    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    bundle(config, &mut writer)?;
    writer.flush()?;
    Ok(())
}
