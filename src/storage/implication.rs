use crate::entities::{Implication, Tag, TagId, TagIdValueIdPair, Value, ValueId};
use crate::errors::*;
use crate::storage::{Row, SqlBuilder, Transaction};

pub fn implications(tx: &mut Transaction) -> Result<Vec<Implication>> {
    let sql = "
SELECT tag.id, tag.name,
       value.id, value.name,
       implied_tag.id, implied_tag.name,
       implied_value.id, implied_value.name
FROM implication
INNER JOIN tag tag ON implication.tag_id = tag.id
LEFT OUTER JOIN value value ON implication.value_id = value.id
INNER JOIN tag implied_tag ON implication.implied_tag_id = implied_tag.id
LEFT OUTER JOIN value implied_value ON implication.implied_value_id = implied_value.id
ORDER BY tag.name, value.name, implied_tag.name, implied_value.name";

    tx.query_vec(sql, parse_implication)
}

pub fn implications_for(
    tx: &mut Transaction,
    pairs: &[TagIdValueIdPair],
) -> Result<Vec<Implication>> {
    let mut builder = SqlBuilder::new();

    builder.append_sql(
        "
SELECT tag.id, tag.name,
       value.id, value.name,
       implied_tag.id, implied_tag.name,
       implied_value.id, implied_value.name
FROM implication
INNER JOIN tag tag ON implication.tag_id = tag.id
LEFT OUTER JOIN value value ON implication.value_id = value.id
INNER JOIN tag implied_tag ON implication.implied_tag_id = implied_tag.id
LEFT OUTER JOIN value implied_value ON implication.implied_value_id = implied_value.id
WHERE ",
    );

    for (index, pair) in pairs.iter().enumerate() {
        if index > 0 {
            builder.append_sql("   OR ");
        }

        builder.append_sql("(implication.tag_id = ");
        builder.append_param(&pair.tag_id);
        builder.append_sql(" AND implication.value_id IN (0");
        if let Some(val_id) = &pair.value_id {
            builder.append_sql(",");
            builder.append_param(val_id);
        }
        builder.append_sql("))");
    }

    builder.append_sql("ORDER BY tag.name, value.name, implied_tag.name, implied_value.name");

    tx.query_vec_params(&builder.sql(), builder.params(), parse_implication)
}

fn parse_implication(row: Row) -> Result<Implication> {
    let implying_tag = Tag {
        id: row.get(0)?,
        name: row.get(1)?,
    };
    // A value ID of 0 means no value (!)
    let implying_value_id: Option<ValueId> = row.get(2)?;
    let implying_value = match implying_value_id {
        None => None,
        Some(id) => Some(Value {
            id,
            name: row.get(3)?,
        }),
    };
    let implied_tag = Tag {
        id: row.get(4)?,
        name: row.get(5)?,
    };
    // A value ID of 0 means no value (!)
    let implied_value_id: Option<ValueId> = row.get(6)?;
    let implied_value = match implied_value_id {
        None => None,
        Some(id) => Some(Value {
            id,
            name: row.get(7)?,
        }),
    };

    Ok(Implication {
        implying_tag,
        implying_value,
        implied_tag,
        implied_value,
    })
}

pub fn add_implication(
    tx: &mut Transaction,
    implying: &TagIdValueIdPair,
    implied: &TagIdValueIdPair,
) -> Result<usize> {
    let sql = "
INSERT OR IGNORE INTO implication (tag_id, value_id, implied_tag_id, implied_value_id)
VALUES (?1, ?2, ?3, ?4)";

    let implying_value_id = implying.value_id.unwrap_or(ValueId(0));
    let implied_value_id = implied.value_id.unwrap_or(ValueId(0));
    let params = rusqlite::params![
        implying.tag_id,
        implying_value_id,
        implied.tag_id,
        implied_value_id
    ];
    tx.execute_params(sql, params)
}

pub fn delete_implication(
    tx: &mut Transaction,
    implying: &TagIdValueIdPair,
    implied: &TagIdValueIdPair,
) -> Result<()> {
    let sql = "
DELETE FROM implication
WHERE tag_id = ?1 AND
      value_id = ?2 AND
      implied_tag_id = ?3 AND
      implied_value_id = ?4";

    let implying_value_id = implying.value_id.unwrap_or(ValueId(0));
    let implied_value_id = implied.value_id.unwrap_or(ValueId(0));
    let params = rusqlite::params![
        implying.tag_id,
        implying_value_id,
        implied.tag_id,
        implied_value_id
    ];
    match tx.execute_params(sql, params) {
        Ok(0) => Err(format!(
            "no such implication where {:?} implies {:?}",
            &implying, &implied
        )
        .into()),
        Ok(1) => Ok(()),
        Ok(_) => Err("expected exactly one row to be affected".into()),
        Err(e) => Err(e),
    }
}

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
