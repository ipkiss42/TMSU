use crate::entities::{Value, ValueId};
use crate::errors::*;
use crate::storage::{self, Row, Transaction};

pub fn value_count(tx: &mut Transaction) -> Result<u64> {
    tx.count_from_table("value")
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

    fn parse_value(row: Row) -> Result<Value> {
        Ok(Value {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    }

    tx.query_vec_params(&sql, &params, parse_value)
}

pub fn value_by_name(tx: &mut Transaction, name: &str) -> Result<Option<Value>> {
    // TODO: figure out why this is needed and if name should be Option<&str> instead
    if name == "" {
        return Ok(Some(Value {
            id: ValueId(0),
            name: "".to_owned(),
        }));
    }

    let results = values_by_names(tx, &[name])?;
    Ok(results.into_iter().next())
}

pub fn rename_value(tx: &mut Transaction, value_id: &ValueId, name: &str) -> Result<()> {
    value_id.assert_non_zero("Bug: renaming a value with ID 0 is meaningless.");

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
