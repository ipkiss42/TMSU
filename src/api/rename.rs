use std::path::Path;

use error_chain::ensure;

use crate::api;
use crate::entities;
use crate::errors::*;
use crate::storage::{self, Storage};

pub fn run_rename_tag(db_path: &Path, curr_name: &str, new_name: &str) -> Result<()> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    let curr_tag = api::load_existing_tag(&mut tx, curr_name)?;

    map_err(
        entities::validate_tag_name(new_name),
        "rename tag",
        curr_name,
        new_name,
    )?;

    let new_tag = storage::tag::tag_by_name(&mut tx, new_name)?;
    ensure!(new_tag.is_none(), "tag '{}' already exists", new_name);

    info!("Renaming tag '{}' to '{}'", curr_name, new_name);

    map_err(
        storage::tag::rename_tag(&mut tx, &curr_tag.id, new_name),
        "rename tag",
        curr_name,
        new_name,
    )?;

    tx.commit()
}

pub fn run_rename_value(db_path: &Path, curr_name: &str, new_name: &str) -> Result<()> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    let curr_value = api::load_existing_value(&mut tx, curr_name)?;

    map_err(
        entities::validate_value_name(new_name),
        "rename value",
        curr_name,
        new_name,
    )?;

    let new_value = storage::value::value_by_name(&mut tx, new_name)?;
    ensure!(new_value.is_none(), "value '{}' already exists", new_name);

    info!("Renaming value '{}' to '{}'", curr_name, new_name);

    map_err(
        storage::value::rename_value(&mut tx, &curr_value.id, new_name),
        "rename value",
        curr_name,
        new_name,
    )?;

    tx.commit()
}

// Helper function to wrap the error message
// XXX: it would probably be simpler to do that only once in the CLI layer, but for some reason the
// Go implementation does not do it for all errors (e.g. when the current tag does not exist)
fn map_err(result: Result<()>, what: &str, curr_name: &str, new_name: &str) -> Result<()> {
    Ok(result.map_err(|e| {
        format!(
            "could not {} '{}' to '{}': {}",
            what, curr_name, new_name, e
        )
    })?)
}
