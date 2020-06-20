use crate::entities::{Tag, TagFileCount, TagId};
use crate::errors::*;
use crate::storage::{self, Row, Transaction};

pub fn tag_count(tx: &mut Transaction) -> Result<u64> {
    tx.count_from_table("tag")
}

pub fn tags_by_names(tx: &mut Transaction, names: &[&str]) -> Result<Vec<Tag>> {
    if names.is_empty() {
        return Ok(vec![]);
    }

    let (placeholders, params) = storage::generate_placeholders(names)?;

    let sql = format!(
        "
SELECT id, name
FROM tag
WHERE name IN ({})",
        &placeholders
    );

    fn parse_tag(row: Row) -> Result<Tag> {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    }

    tx.query_vec_params(&sql, &params, parse_tag)
}

pub fn tag_by_name(tx: &mut Transaction, name: &str) -> Result<Option<Tag>> {
    // TODO: figure out why this is needed and if name should be Option<&str> instead
    if name == "" {
        return Ok(Some(Tag {
            id: TagId(0),
            name: "".to_owned(),
        }));
    }

    let results = tags_by_names(tx, &[name])?;
    Ok(results.into_iter().next())
}

pub fn insert_tag(tx: &mut Transaction, name: &str) -> Result<Tag> {
    let sql = "
INSERT INTO tag (name)
VALUES (?)";

    let params = rusqlite::params![name];
    tx.execute_params(sql, params)?;

    let tag_id = tx.last_inserted_row_id();
    Ok(Tag {
        id: TagId(tag_id),
        name: name.to_owned(),
    })
}

pub fn rename_tag(tx: &mut Transaction, tag_id: &TagId, name: &str) -> Result<()> {
    let sql = "
UPDATE tag
SET name = ?
WHERE id = ?";

    let params = rusqlite::params![name, tag_id];
    match tx.execute_params(sql, params) {
        Ok(1) => Ok(()),
        Ok(_) => Err("Expected exactly one row to be affected".into()),
        Err(e) => Err(e),
    }
}

pub fn delete_tag(tx: &mut Transaction, tag_id: &TagId) -> Result<()> {
    let sql = "
DELETE FROM tag
WHERE id = ?";

    let params = rusqlite::params![tag_id];
    match tx.execute_params(sql, params) {
        // Note: this is stricter than the Go version, which does not fail when no row is deleted
        Ok(1) => Ok(()),
        Ok(_) => Err("Expected exactly one row to be affected".into()),
        Err(e) => Err(e),
    }
}

/// Retrieve the usage (file count) of each tag
pub fn tag_usage(tx: &mut Transaction) -> Result<Vec<TagFileCount>> {
    let sql = "
SELECT t.id, t.name, count(file_id)
FROM file_tag ft, tag t
WHERE ft.tag_id = t.id
GROUP BY t.id
ORDER BY t.name";

    fn parse_row(row: Row) -> Result<TagFileCount> {
        Ok(TagFileCount {
            id: row.get(0)?,
            name: row.get(1)?,
            file_count: row.get(2)?,
        })
    }

    tx.query_vec(sql, parse_row)
}
