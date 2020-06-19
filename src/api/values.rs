use std::path::Path;

use crate::api;
use crate::errors::*;
use crate::storage::{self, Storage, Transaction};

pub struct ValuesOutput {
    pub value_groups: Vec<ValueGroup>,
}

/// One group of values. If the tag name is present, then the values correspond to the tag.
pub struct ValueGroup {
    pub tag_name: Option<String>,
    pub value_names: Vec<String>,
}

/// If `tag_names` is empty, then all the values are returned in one `ValueGroup`.
/// Otherwise, the output contains one `ValueGroup` per tag.
pub fn run_values(db_path: &Path, tag_names: &[&str]) -> Result<ValuesOutput> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    if tag_names.is_empty() {
        Ok(list_all_values(&mut tx)?)
    } else {
        Ok(list_values_for_tags(&mut tx, tag_names)?)
    }
}

fn list_all_values(tx: &mut Transaction) -> Result<ValuesOutput> {
    info!("Retrieving all values");

    let value_names = storage::value::values(tx)?
        .iter()
        .map(|v| v.name.to_owned())
        .collect();

    Ok(ValuesOutput {
        value_groups: vec![ValueGroup {
            tag_name: None,
            value_names,
        }],
    })
}

fn list_values_for_tags(tx: &mut Transaction, tag_names: &[&str]) -> Result<ValuesOutput> {
    let mut value_groups = Vec::with_capacity(tag_names.len());

    for tag_name in tag_names {
        let tag = api::load_existing_tag(tx, tag_name)?;

        info!("Retrieving values for tag '{}'", tag_name);
        let value_names = storage::value::values_by_tag_id(tx, &tag.id)?
            .iter()
            .map(|v| v.name.to_owned())
            .collect();

        value_groups.push(ValueGroup {
            tag_name: Some((*tag_name).to_owned()),
            value_names,
        });
    }
    Ok(ValuesOutput { value_groups })
}
