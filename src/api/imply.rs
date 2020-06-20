use std::collections::HashSet;
use std::fmt;
use std::path::Path;

use crate::api;
use crate::entities::{self, ValueId};
use crate::errors::*;
use crate::storage::{self, Storage, Transaction};

pub struct ImplyListOutput {
    pub implications: Vec<Implication>,
}

#[derive(Debug)]
pub struct Implication {
    pub implying: TagAndOptionalValue,
    pub implied: TagAndOptionalValue,
}

#[derive(Debug)]
pub struct TagAndOptionalValue {
    pub tag_name: String,
    pub value_name: Option<String>,
}

impl fmt::Display for TagAndOptionalValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value_name {
            None => write!(f, "{}", &self.tag_name),
            Some(val) => write!(f, "{}={}", &self.tag_name, val),
        }
    }
}

pub fn run_imply_list(db_path: &Path) -> Result<ImplyListOutput> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    let implications = storage::implication::implications(&mut tx)?;

    tx.commit()?;

    let converted = implications.into_iter().map(convert_to_output).collect();

    Ok(ImplyListOutput {
        implications: converted,
    })
}

fn convert_to_output(implication: entities::Implication) -> Implication {
    Implication {
        implying: TagAndOptionalValue {
            tag_name: implication.implying_tag.name,
            value_name: implication.implying_value.map(|v| v.name),
        },
        implied: TagAndOptionalValue {
            tag_name: implication.implied_tag.name,
            value_name: implication.implied_value.map(|v| v.name),
        },
    }
}

pub fn delete_implications(db_path: &Path, implications: &[Implication]) -> Result<()> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    for implication in implications {
        info!(
            "Removing tag implication: '{}' -> '{}'",
            &implication.implying, &implication.implied
        );

        let implying_pair = convert_to_id_pair_no_create(&mut tx, &implication.implying)?;
        let implied_pair = convert_to_id_pair_no_create(&mut tx, &implication.implied)?;

        storage::implication::delete_implication(&mut tx, &implying_pair, &implied_pair).map_err(
            |e| {
                format!(
                    "could not delete implication of '{}' to '{}': {}",
                    &implication.implying, implication.implied, e
                )
            },
        )?;
    }

    tx.commit()
}

fn convert_to_id_pair_no_create(
    tx: &mut Transaction,
    tag_and_value: &TagAndOptionalValue,
) -> Result<entities::TagIdValueIdPair> {
    let tag_id = api::load_existing_tag(tx, &tag_and_value.tag_name)?.id;
    let value = tag_and_value
        .value_name
        .as_ref()
        .map(|vn| api::load_existing_value(tx, &vn))
        .transpose()?;

    Ok(entities::TagIdValueIdPair {
        tag_id,
        value_id: value.map(|v| v.id),
    })
}

pub fn add_implications(db_path: &Path, implications: &[Implication]) -> Result<()> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    info!("Loading settings");
    let settings = storage::setting::settings(&mut tx)?;

    for implication in implications {
        let implying_pair =
            convert_to_id_pair_may_create(&mut tx, &implication.implying, &settings)?;
        let implied_pair = convert_to_id_pair_may_create(&mut tx, &implication.implied, &settings)?;

        info!(
            "Adding tag implication: '{}' -> '{}'",
            &implication.implying, &implication.implied
        );

        add_single_implication(&mut tx, &implying_pair, &implied_pair).map_err(|e| {
            format!(
                "could not add implication of '{}' to '{}': {}",
                &implication.implying, implication.implied, e
            )
        })?;
    }

    tx.commit()
}

/// Simple auxiliary function, used only to avoid duplicating the "map_err" in the calling code.
fn add_single_implication(
    tx: &mut Transaction,
    implying_pair: &entities::TagIdValueIdPair,
    implied_pair: &entities::TagIdValueIdPair,
) -> Result<()> {
    check_for_implication_cycles(tx, &implying_pair, &implied_pair)?;

    storage::implication::add_implication(tx, &implying_pair, &implied_pair)?;

    Ok(())
}

fn convert_to_id_pair_may_create(
    tx: &mut Transaction,
    tag_and_value: &TagAndOptionalValue,
    settings: &entities::settings::Settings,
) -> Result<entities::TagIdValueIdPair> {
    let tag = api::load_or_create_tag(tx, &tag_and_value.tag_name, settings)?;
    let value = tag_and_value
        .value_name
        .as_ref()
        .map(|vn| api::load_or_create_value(tx, &vn, settings))
        .transpose()?;

    Ok(entities::TagIdValueIdPair {
        tag_id: tag.id,
        value_id: value.map(|v| v.id),
    })
}

fn check_for_implication_cycles(
    tx: &mut Transaction,
    implying: &entities::TagIdValueIdPair,
    implied: &entities::TagIdValueIdPair,
) -> Result<()> {
    let implications = transitive_implications_for(tx, implied)?;

    for implication in implications {
        if implication.implied_tag.id == implying.tag_id
            && (implying.value_id.is_none()
                || equal_values(&implying.value_id, &implication.implied_value))
        {
            return Err("implication would create a cycle".into());
        }
    }
    Ok(())
}

fn transitive_implications_for(
    tx: &mut Transaction,
    initial_pair: &entities::TagIdValueIdPair,
) -> Result<Vec<entities::Implication>> {
    let mut resultant_implications = HashSet::new();

    let mut to_process = vec![initial_pair.clone()];

    while !to_process.is_empty() {
        let implications = storage::implication::implications_for(tx, &to_process)?;

        to_process = Vec::new();
        for implication in implications {
            if !resultant_implications.contains(&implication) {
                to_process.push(entities::TagIdValueIdPair {
                    tag_id: implication.implied_tag.id,
                    value_id: implication.implied_value.as_ref().map(|v| v.id),
                });
                resultant_implications.insert(implication);
            }
        }
    }

    Ok(resultant_implications.into_iter().collect())
}

fn equal_values(val1: &Option<ValueId>, val2: &Option<entities::Value>) -> bool {
    if let Some(id1) = *val1 {
        if let Some(v2) = val2 {
            return id1 == v2.id;
        }
    }

    false
}
