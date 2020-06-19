use crate::entities::{TagId, ValueId};
use crate::errors::*;
use crate::storage::Transaction;

pub fn delete_implications_by_tag_id(tx: &mut Transaction, tag_id: &TagId) -> Result<usize> {
    let sql = "
DELETE FROM implication
WHERE tag_id = ?1 OR implied_tag_id = ?1";

    let params = rusqlite::params![tag_id];
    tx.execute_params(sql, params)
}

pub fn delete_implications_by_value_id(tx: &mut Transaction, value_id: &ValueId) -> Result<usize> {
    let sql = "
DELETE FROM implication
WHERE value_id = ?1 OR implied_value_id = ?1";

    let params = rusqlite::params![value_id];
    tx.execute_params(sql, params)
}
