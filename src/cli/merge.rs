use lazy_static::lazy_static;
use structopt::StructOpt;

use crate::api;
use crate::cli::{extract_names, locate_db, GlobalOptions, TagOrValueName};
use crate::errors::*;

lazy_static! {
    static ref EXAMPLES: String = super::generate_examples(&[
        ("tmsu merge cehese cheese", None),
        ("tmsu merge outdoors outdoor outside", None)
    ]);
}

/// Merges TAGs into tag DEST resulting in a single tag of name DEST.
#[derive(Debug, StructOpt)]
#[structopt(after_help(EXAMPLES.as_str()))]
pub struct MergeOptions {
    /// Merges values
    #[structopt(short, long)]
    value: bool,

    /// Tag(s) or value(s) to merge
    #[structopt(required = true)]
    names: Vec<TagOrValueName>,

    /// Destination tag or value
    #[structopt()]
    dest: TagOrValueName,
}

impl MergeOptions {
    pub fn execute(&self, global_opts: &GlobalOptions) -> Result<()> {
        let db_path = locate_db(&global_opts.database)?;
        info!("Database path: {}", db_path.display());

        let names = extract_names(&self.names);

        if self.value {
            api::merge::run_merge_values(&db_path, &names, &self.dest.name)?;
        } else {
            api::merge::run_merge_tags(&db_path, &names, &self.dest.name)?;
        }

        Ok(())
    }
}
