use std::path::Path;

use error_chain::ensure;

use crate::api;
use crate::entities;
use crate::errors::*;
use crate::storage::{self, Storage, Transaction};

pub fn run_copy(db_path: &Path, curr_tag_name: &str, dest_tag_names: &[&str]) -> Result<()> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    let curr_tag = api::load_existing_tag(&mut tx, curr_tag_name)?;

    for dest_tag_name in dest_tag_names {
        let tag = storage::tag::tag_by_name(&mut tx, dest_tag_name)?;
        ensure!(
            tag.is_none(),
            format!("a tag with name '{}' already exists", dest_tag_name)
        );

        info!("copying tag '{}' to '{}'", curr_tag_name, dest_tag_name);
        copy_tag(&mut tx, &curr_tag.id, dest_tag_name).map_err(|e| {
            format!(
                "could not copy tag '{}' to '{}': {}",
                curr_tag_name, dest_tag_name, e
            )
        })?;
    }

    tx.commit()
}

fn copy_tag(
    tx: &mut Transaction,
    curr_tag_id: &entities::TagId,
    dest_tag_name: &str,
) -> Result<()> {
    entities::validate_tag_name(dest_tag_name)?;

    let new_tag = storage::tag::insert_tag(tx, dest_tag_name)?;
    storage::filetag::copy_file_tags(tx, curr_tag_id, &new_tag.id)?;

    Ok(())
}
