use chrono::DateTime;

use crate::entities::{path::ScopedPath, File, FileId};
use crate::errors::*;
use crate::storage::{os_to_str, Row, Transaction};

const TIMESTAMP_FORMAT: &str = "%F %T%.f%:z";

pub fn file_count(tx: &mut Transaction) -> Result<u64> {
    tx.count_from_table("file")
}

pub fn file_by_path(tx: &mut Transaction, scoped_path: &ScopedPath) -> Result<Option<File>> {
    let sql = "
SELECT id, directory, name, fingerprint, mod_time, size, is_dir
FROM file
WHERE directory = ? AND name = ?";

    let (dir, name) = scoped_path.inner_as_dir_and_name();

    let params = rusqlite::params![os_to_str(&dir)?, os_to_str(&name)?];
    tx.query_single_params(sql, params, parse_file)
}

fn parse_file(row: Row) -> Result<File> {
    let mod_time_str: String = row.get(4)?;
    let mod_time = DateTime::parse_from_str(&mod_time_str, TIMESTAMP_FORMAT)?;

    Ok(File {
        id: row.get(0)?,
        dir: row.get(1)?,
        name: row.get(2)?,
        fingerprint: row.get(3)?,
        mod_time,
        size: row.get_usize(5)?,
        is_dir: row.get(6)?,
    })
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
