use crate::entities::FileId;
use crate::errors::*;
use crate::storage::Transaction;

pub fn file_count(tx: &mut Transaction) -> Result<u64> {
    tx.count_from_table("file")
}

pub fn delete_untagged_files(tx: &mut Transaction, file_ids: &[FileId]) -> Result<()> {
    let sql = "
DELETE FROM file
WHERE id = ?1
AND (SELECT count(1)
     FROM file_tag
     WHERE file_id = ?1) == 0";

    for file_id in file_ids {
        let params = rusqlite::params![file_id];
        tx.execute_params(sql, params)?;
    }

    Ok(())
}
