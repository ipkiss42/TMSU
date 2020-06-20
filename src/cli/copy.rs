use lazy_static::lazy_static;
use structopt::StructOpt;

use crate::api;
use crate::cli::{extract_names, locate_db, GlobalOptions, TagOrValueName};
use crate::errors::*;

lazy_static! {
    static ref EXAMPLES: String = super::generate_examples(&[
        ("tmsu copy cheese wine", None),
        ("tmsu copy report document text", None)
    ]);
}

/// Creates a new tag NEW applied to the same set of files as TAG.
#[derive(Debug, StructOpt)]
#[structopt(after_help(EXAMPLES.as_str()))]
pub struct CopyOptions {
    /// Old tag or value
    #[structopt()]
    tag: TagOrValueName,

    /// New tag or value
    #[structopt(required = true)]
    new: Vec<TagOrValueName>,
}

impl CopyOptions {
    pub fn execute(&self, global_opts: &GlobalOptions) -> Result<()> {
        let db_path = locate_db(&global_opts.database)?;
        info!("Database path: {}", db_path.display());

        api::copy::run_copy(&db_path, &self.tag.name, &extract_names(&self.new))
    }
}
