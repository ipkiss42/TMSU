pub mod config;
pub mod copy;
pub mod delete;
pub mod imply;
pub mod info;
pub mod init;
pub mod merge;
pub mod rename;
pub mod tags;
pub mod values;

use error_chain::ensure;

use crate::entities::{self, settings::Settings, Tag, Value};
use crate::errors::*;
use crate::storage::{self, Transaction};

fn load_existing_tag(tx: &mut Transaction, name: &str) -> Result<Tag> {
    let tag_opt = storage::tag::tag_by_name(tx, name)?;
    ensure!(tag_opt.is_some(), "no such tag '{}'", name);
    // Safe to unwrap, since we just checked it
    // TODO: check if there is a more idiomatic way, without a full-blown pattern match
    Ok(tag_opt.unwrap())
}

fn load_or_create_tag(tx: &mut Transaction, name: &str, settings: &Settings) -> Result<Tag> {
    let tag_opt = storage::tag::tag_by_name(tx, name)?;
    match tag_opt {
        Some(tag) => Ok(tag),
        None => {
            ensure!(
                settings.auto_create_tags(),
                format!("no such tag '{}'", name)
            );

            entities::validate_tag_name(name)?;

            let tag = storage::tag::insert_tag(tx, name)?;
            warn!("new tag '{}'", name);
            Ok(tag)
        }
    }
}

fn load_existing_value(tx: &mut Transaction, name: &str) -> Result<Value> {
    let value_opt = storage::value::value_by_name(tx, name)?;
    ensure!(value_opt.is_some(), "no such value '{}'", name);
    // Safe to unwrap, since we just checked it
    // TODO: check if there is a more idiomatic way, without a full-blown pattern match
    Ok(value_opt.unwrap())
}

fn load_or_create_value(tx: &mut Transaction, name: &str, settings: &Settings) -> Result<Value> {
    let value_opt = storage::value::value_by_name(tx, name)?;
    match value_opt {
        Some(value) => Ok(value),
        None => {
            ensure!(
                settings.auto_create_values(),
                format!("no such value '{}'", name)
            );

            entities::validate_value_name(name)?;

            let value = storage::value::insert_value(tx, name)?;
            warn!("new value '{}'", name);
            Ok(value)
        }
    }
}
