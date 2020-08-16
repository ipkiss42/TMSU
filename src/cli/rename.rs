use lazy_static::lazy_static;
use structopt::StructOpt;

use crate::api;
use crate::cli::{locate_db, GlobalOptions, TagOrValueName};
use crate::errors::*;

lazy_static! {
    static ref EXAMPLES: String = super::generate_examples(&[
        ("tmsu rename montain mountain", None),
        ("tmsu rename --value MMXVII 2017", None)
    ]);
}

/// Renames a tag or value from OLD to NEW.
///
/// Attempting to rename a tag or value with a name that already exists will result in an error.
/// To merge tags or values use the merge subcommand instead.
#[derive(Debug, StructOpt)]
#[structopt(after_help(EXAMPLES.as_str()))]
pub struct RenameOptions {
    /// Renames a value
    #[structopt(short, long)]
    value: bool,

    /// Old tag or value
    #[structopt()]
    old: TagOrValueName,

    /// New tag or value
    #[structopt()]
    new: TagOrValueName,
}

impl RenameOptions {
    pub fn execute(&self, global_opts: &GlobalOptions) -> Result<()> {
        let db_path = locate_db(&global_opts.database)?;
        info!("Database path: {}", db_path.display());

        if self.value {
            api::rename::run_rename_value(&db_path, &self.old.name, &self.new.name)?;
        } else {
            api::rename::run_rename_tag(&db_path, &self.old.name, &self.new.name)?;
        }

        Ok(())
    }
}
