use crate::errors::*;
use crate::storage::Transaction;

pub fn value_count(tx: &mut Transaction) -> Result<u64> {
    tx.count_from_table("value")
}
