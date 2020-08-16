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

    entities::validate_tag_name(new_name)?;

    let new_tag = storage::tag::tag_by_name(&mut tx, new_name)?;
    ensure!(new_tag.is_none(), "tag '{}' already exists", new_name);

    info!("Renaming tag '{}' to '{}'", curr_name, new_name);

    storage::tag::rename_tag(&mut tx, &curr_tag.id, new_name).map_err(|e| {
        format!(
            "could not rename tag '{}' to '{}': {}",
            curr_name, new_name, e
        )
    })?;

    tx.commit()
}

pub fn run_rename_value(db_path: &Path, curr_name: &str, new_name: &str) -> Result<()> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    let curr_value = api::load_existing_value(&mut tx, curr_name)?;

    entities::validate_value_name(new_name)?;

    let new_value = storage::value::value_by_name(&mut tx, new_name)?;
    ensure!(new_value.is_none(), "value '{}' already exists", new_name);

    info!("Renaming value '{}' to '{}'", curr_name, new_name);

    storage::value::rename_value(&mut tx, &curr_value.id, new_name).map_err(|e| {
        format!(
            "could not rename value '{}' to '{}': {}",
            curr_name, new_name, e
        )
    })?;

    tx.commit()
}
