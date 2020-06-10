use crate::entities::TagFileCount;
use crate::errors::*;
use crate::storage::{Row, Transaction};

pub fn tag_count(tx: &mut Transaction) -> Result<u64> {
    tx.count_from_table("tag")
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
