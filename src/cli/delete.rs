use lazy_static::lazy_static;
use structopt::StructOpt;

use crate::api;
use crate::cli::{extract_names, locate_db, GlobalOptions, TagOrValueName};
use crate::errors::*;

lazy_static! {
    static ref EXAMPLES: String = super::generate_examples(&[
        ("tmsu delete pineapple", None),
        ("tmsu delete red green blue", None)
    ]);
}

/// Permanently deletes the tags or values specified.
#[derive(Debug, StructOpt)]
#[structopt(after_help(EXAMPLES.as_str()))]
pub struct DeleteOptions {
    /// Deletes a value
    #[structopt(short, long)]
    value: bool,

    /// Tag or value names
    #[structopt(required = true)]
    names: Vec<TagOrValueName>,
}

impl DeleteOptions {
    pub fn execute(&self, global_opts: &GlobalOptions) -> Result<()> {
        let db_path = locate_db(&global_opts.database)?;
        info!("Database path: {}", db_path.display());

        let names = extract_names(&self.names);

        if self.value {
            api::delete::run_delete_value(&db_path, &names)?;
        } else {
            api::delete::run_delete_tag(&db_path, &names)?;
        }

        Ok(())
    }
}
