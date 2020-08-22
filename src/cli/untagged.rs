use std::env;
use std::fs;
use std::path::PathBuf;

use lazy_static::lazy_static;
use structopt::StructOpt;

use crate::api;
use crate::cli::{locate_db, GlobalOptions};
use crate::errors::*;

lazy_static! {
    static ref EXAMPLES: String = super::generate_examples(&[
        ("tmsu untagged", None),
        ("tmsu untagged /home/fred/drawings", None),
    ]);
}

/// Identify untagged files in the filesystem.
///
/// Where PATHs are not specified, untagged items under the current working directory are shown.
#[derive(Debug, StructOpt)]
#[structopt(after_help(EXAMPLES.as_str()))]
pub struct UntaggedOptions {
    /// List the number of files rather than their names
    #[structopt(short("c"), long("count"))]
    show_count: bool,

    /// Do not examine directory contents (non-recursive)
    #[structopt(short, long)]
    directory: bool,

    /// Do not dereference symbolic links
    #[structopt(short("P"), long)]
    no_dereference: bool,

    /// File paths
    #[structopt()]
    paths: Vec<PathBuf>,
}

impl UntaggedOptions {
    pub fn execute(&self, global_opts: &GlobalOptions) -> Result<()> {
        let db_path = locate_db(&global_opts.database)?;
        info!("Database path: {}", db_path.display());

        let mut paths = self.paths.clone();
        if paths.is_empty() {
            // Default to the current directory
            paths = get_curr_dir_entries()?;
        }

        if self.show_count {
            let mut counter = 0;

            api::untagged::list_untagged_for_paths(
                &db_path,
                &paths,
                !self.directory,
                !self.no_dereference,
                &mut |_path| {
                    counter += 1;
                },
            )?;

            println!("{}", counter);
        } else {
            let cwd = env::current_dir()?;
            api::untagged::list_untagged_for_paths(
                &db_path,
                &paths,
                !self.directory,
                !self.no_dereference,
                &mut |path| {
                    // FIXME: this should be relative to the CWD, not to the DB path
                    println!("{}", super::rel_to(&path, &cwd).display());
                },
            )?;
        }
        Ok(())
    }
}

fn get_curr_dir_entries() -> Result<Vec<PathBuf>> {
    let mut entries = vec![];
    for entry in fs::read_dir(".")? {
        entries.push(entry?.path());
    }

    Ok(entries)
}
