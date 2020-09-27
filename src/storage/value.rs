use crate::entities::{TagId, Value, ValueId};
use crate::errors::*;
use crate::storage::{self, Row, Transaction};

pub fn value_count(tx: &mut Transaction) -> Result<u64> {
    tx.count_from_table("value")
}

pub fn values(tx: &mut Transaction) -> Result<Vec<Value>> {
    let sql = "
SELECT id, name
FROM value
ORDER BY name";

    tx.query_vec(sql, parse_value)
}

pub fn values_by_names(tx: &mut Transaction, names: &[&str]) -> Result<Vec<Value>> {
    if names.is_empty() {
        return Ok(vec![]);
    }

    let (placeholders, params) = storage::generate_placeholders(names)?;

    let sql = format!(
        "
SELECT id, name
FROM value
WHERE name IN ({})",
        &placeholders
    );

    tx.query_vec_params(&sql, &params, parse_value)
}

pub fn value_by_name(tx: &mut Transaction, name: &str) -> Result<Option<Value>> {
    // Note: when the name is an empty string, the Go implementation returns a
    // Value with an ID of 0. While it is unnecessary in most cases due to checks in upper layers,
    // it is still relied upon in some subcommands, such as "imply" and "tag".
    // We don't replicate this here, since we have stronger typing thanks to
    // entities::OptionalValueId.
    let results = values_by_names(tx, &[name])?;
    Ok(results.into_iter().next())
}

pub fn values_by_tag_id(tx: &mut Transaction, tag_id: &TagId) -> Result<Vec<Value>> {
    let sql = "
SELECT id, name
FROM value
WHERE id IN (SELECT value_id
             FROM file_tag
             WHERE tag_id = ?1)
ORDER BY name";

    let params = rusqlite::params![tag_id];
    tx.query_vec_params(sql, params, parse_value)
}

fn parse_value(row: Row) -> Result<Value> {
    Ok(Value {
        id: row.get(0)?,
        name: row.get(1)?,
    })
}

pub fn rename_value(tx: &mut Transaction, value_id: &ValueId, name: &str) -> Result<()> {
    let sql = "
UPDATE value
SET name = ?
WHERE id = ?";

    let params = rusqlite::params![name, value_id];
    match tx.execute_params(sql, params) {
        Ok(1) => Ok(()),
        Ok(_) => Err("Expected exactly one row to be affected".into()),
        Err(e) => Err(e),
    }
}

pub fn delete_value(tx: &mut Transaction, value_id: &ValueId) -> Result<()> {
    let sql = "
DELETE FROM value
WHERE id = ?";

    let params = rusqlite::params![value_id];
    match tx.execute_params(sql, params) {
        // Note: this is stricter than the Go version, which does not fail when no row is deleted
        Ok(1) => Ok(()),
        Ok(_) => Err("Expected exactly one row to be affected".into()),
        Err(e) => Err(e),
    }
}
