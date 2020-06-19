use std::path::Path;

use crate::api;
use crate::errors::*;
use crate::storage::{self, Storage};

pub fn run_merge_tags(db_path: &Path, source_names: &[&str], dest_name: &str) -> Result<()> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    let dest_tag = api::load_existing_tag(&mut tx, dest_name)?;

    for source_name in source_names {
        if *source_name == dest_name {
            return Err(format!("cannot merge tag '{}' into itself", source_name).into());
        }

        let source_tag = api::load_existing_tag(&mut tx, source_name)?;

        info!("Finding files tagged '{}'", source_name);
        let file_tags = storage::filetag::file_tags_by_tag_id(&mut tx, &source_tag.id)?;

        info!("Applying tag '{}' to these files", dest_name);
        for file_tag in file_tags {
            storage::filetag::add_file_tag(
                &mut tx,
                file_tag.file_id,
                dest_tag.id,
                file_tag.value_id,
            )
            .map_err(|e| {
                format!(
                    "could not apply tag '{}' to file '{}': {}",
                    dest_name, file_tag.file_id, e
                )
            })?;
        }

        info!("Deleting tag '{}'", source_name);
        api::delete::delete_tag(&mut tx, &source_tag)
            .map_err(|e| format!("could not delete tag '{}': {}", source_name, e))?;
    }

    tx.commit()
}

pub fn run_merge_values(db_path: &Path, source_names: &[&str], dest_name: &str) -> Result<()> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    let dest_value = api::load_existing_value(&mut tx, dest_name)?;

    for source_name in source_names {
        if *source_name == dest_name {
            return Err(format!("cannot merge value '{}' into itself", source_name).into());
        }

        let source_value = api::load_existing_value(&mut tx, source_name)?;

        info!("Finding files tagged '{}'", source_name);
        let file_tags = storage::filetag::file_tags_by_value_id(&mut tx, &source_value.id)?;

        info!("Applying value '{}' to these files", dest_name);
        for file_tag in file_tags {
            storage::filetag::add_file_tag(
                &mut tx,
                file_tag.file_id,
                file_tag.tag_id,
                Some(dest_value.id),
            )
            .map_err(|e| {
                format!(
                    "could not apply value '{}' to file '{}': {}",
                    dest_name, file_tag.file_id, e
                )
            })?;
        }

        info!("Deleting value '{}'", source_name);
        api::delete::delete_value(&mut tx, &source_value)
            .map_err(|e| format!("could not delete value '{}': {}", source_name, e))?;
    }

    tx.commit()
}
