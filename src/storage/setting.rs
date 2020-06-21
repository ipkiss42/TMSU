use crate::entities::settings::Settings;
use crate::errors::*;
use crate::storage::{Row, Transaction};

pub fn settings(tx: &mut Transaction) -> Result<Settings> {
    // Get the default settings
    let mut settings = Settings::new();

    let sql = "
SELECT name, value
FROM setting";

    let db_settings = tx.query_vec(sql, parse_setting)?;
    // Override the default settings with the ones from DB
    for (name, value) in db_settings {
        // Explicitly ignore settings from the DB which are invalid.
        // This differs from the Go implementation.
        settings.set(&name, &value).ok();
    }

    Ok(settings)
}

pub fn update_setting(tx: &mut Transaction, name: &str, value: &str) -> Result<usize> {
    let sql = "
INSERT OR REPLACE INTO setting (name, value)
VALUES (?, ?)";

    let params = rusqlite::params![name, value];
    tx.execute_params(sql, params)
}

fn parse_setting(row: Row) -> Result<(String, String)> {
    Ok((row.get(0)?, row.get(1)?))
}
