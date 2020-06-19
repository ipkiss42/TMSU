use crate::entities::{FileTag, TagId, ValueId};
use crate::errors::*;
use crate::storage::{Row, Transaction};

pub fn file_tag_count(tx: &mut Transaction) -> Result<u64> {
    tx.count_from_table("file_tag")
}

pub fn file_tags_by_tag_id(tx: &mut Transaction, tag_id: &TagId) -> Result<Vec<FileTag>> {
    let sql = "
SELECT file_id, tag_id, value_id
FROM file_tag
WHERE tag_id = ?1";

    let params = rusqlite::params![tag_id];
    tx.query_vec_params(sql, params, parse_file_tag)
}

pub fn file_tags_by_value_id(tx: &mut Transaction, value_id: &ValueId) -> Result<Vec<FileTag>> {
    value_id.assert_non_zero("Bug: searching file tags with a value ID of 0 is meaningless.");

    let sql = "
SELECT file_id, tag_id, value_id
FROM file_tag
WHERE value_id = ?1";

    let params = rusqlite::params![value_id];
    tx.query_vec_params(sql, params, parse_file_tag)
}

fn parse_file_tag(row: Row) -> Result<FileTag> {
    Ok(FileTag {
        file_id: row.get(0)?,
        tag_id: row.get(1)?,
        // A value ID of 0 in the DB actually means no value...
        value_id: match row.get(2)? {
            0 => None,
            id => Some(ValueId(id)),
        },
        explicit: true,
        implicit: false,
    })
}

pub fn delete_file_tags_by_tag_id(tx: &mut Transaction, tag_id: &TagId) -> Result<usize> {
    let sql = "
DELETE FROM file_tag
WHERE tag_id = ?";

    let params = rusqlite::params![tag_id];
    tx.execute_params(sql, params)
}

pub fn delete_file_tags_by_value_id(tx: &mut Transaction, value_id: &ValueId) -> Result<usize> {
    value_id.assert_non_zero("Bug: deleting file tags with a value ID of 0 is meaningless.");

    let sql = "
DELETE FROM file_tag
WHERE value_id = ?";

    let params = rusqlite::params![value_id];
    tx.execute_params(sql, params)
}
