use crate::errors::*;
use crate::storage::Transaction;

pub fn file_tag_count(tx: &mut Transaction) -> Result<u64> {
    tx.count_from_table("file_tag")
}
