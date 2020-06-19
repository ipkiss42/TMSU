use std::path::Path;

use crate::api;
use crate::entities::{FileId, FileTag, Tag, Value};
use crate::errors::*;
use crate::storage::{self, Storage, Transaction};

pub fn run_delete_tag(db_path: &Path, tag_names: &[&str]) -> Result<()> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    for name in tag_names {
        let tag = api::load_existing_tag(&mut tx, name)?;

        info!("Deleting tag '{}'", name);

        delete_tag(&mut tx, &tag).map_err(|e| format!("could not delete tag '{}': {}", name, e))?;
    }

    tx.commit()
}

pub fn run_delete_value(db_path: &Path, value_names: &[&str]) -> Result<()> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    for name in value_names {
        let value = api::load_existing_value(&mut tx, name)?;

        info!("Deleting value '{}'", name);

        delete_value(&mut tx, &value)
            .map_err(|e| format!("could not delete value '{}': {}", name, e))?;
    }

    tx.commit()
}

pub fn delete_tag(tx: &mut Transaction, tag: &Tag) -> Result<()> {
    delete_file_tags_by_tag_id(tx, tag)?;
    storage::implication::delete_implications_by_tag_id(tx, &tag.id)?;
    storage::tag::delete_tag(tx, &tag.id)
}

pub fn delete_value(tx: &mut Transaction, value: &Value) -> Result<()> {
    delete_file_tags_by_value_id(tx, value)?;
    storage::implication::delete_implications_by_value_id(tx, &value.id)?;
    storage::value::delete_value(tx, &value.id)
}

fn delete_file_tags_by_tag_id(tx: &mut Transaction, tag: &Tag) -> Result<()> {
    let file_tags = storage::filetag::file_tags_by_tag_id(tx, &tag.id)?;
    storage::filetag::delete_file_tags_by_tag_id(tx, &tag.id)?;
    let file_ids = extract_file_ids(&file_tags);
    storage::file::delete_untagged_files(tx, &file_ids)
}

fn delete_file_tags_by_value_id(tx: &mut Transaction, value: &Value) -> Result<()> {
    let file_tags = storage::filetag::file_tags_by_value_id(tx, &value.id)?;
    storage::filetag::delete_file_tags_by_value_id(tx, &value.id)?;
    let file_ids = extract_file_ids(&file_tags);
    storage::file::delete_untagged_files(tx, &file_ids)
}

fn extract_file_ids(file_tags: &[FileTag]) -> Vec<FileId> {
    file_tags.iter().map(|ft| ft.file_id).collect()
}
