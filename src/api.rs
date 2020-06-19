pub mod delete;
pub mod info;
pub mod init;
pub mod rename;

use error_chain::ensure;

use crate::entities::{Tag, Value};
use crate::errors::*;
use crate::storage::{self, Transaction};

fn load_existing_tag(tx: &mut Transaction, name: &str) -> Result<Tag> {
    let tag_opt = storage::tag::tag_by_name(tx, name)?;
    ensure!(tag_opt.is_some(), "no such tag '{}'", name);
    // Safe to unwrap, since we just checked it
    // TODO: check if there is a more idiomatic way, without a full-blown pattern match
    Ok(tag_opt.unwrap())
}

fn load_existing_value(tx: &mut Transaction, name: &str) -> Result<Value> {
    let value_opt = storage::value::value_by_name(tx, name)?;
    ensure!(value_opt.is_some(), "no such value '{}'", name);
    // Safe to unwrap, since we just checked it
    // TODO: check if there is a more idiomatic way, without a full-blown pattern match
    Ok(value_opt.unwrap())
}
