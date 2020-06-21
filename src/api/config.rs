use std::path::Path;

use crate::entities::settings::Settings;
use crate::errors::*;
use crate::storage::{self, Storage};

pub struct Setting {
    pub name: String,
    pub value: String,
}

pub fn run_config_list_all_settings(db_path: &Path) -> Result<Vec<Setting>> {
    let settings = get_all_settings(db_path)?;

    Ok(settings
        .list()
        .iter()
        .map(|s| Setting {
            name: s.name().to_owned(),
            value: s.as_str(),
        })
        .collect())
}

pub fn run_config_get_setting_value(db_path: &Path, name: &str) -> Result<String> {
    let settings = get_all_settings(db_path)?;

    match settings.get(name) {
        Some(setting) => Ok(setting.as_str()),
        None => Err(format!("no such setting '{}'", name).into()),
    }
}

pub fn run_config_update_setting(db_path: &Path, name: &str, value: &str) -> Result<()> {
    // Validate both name and value
    let mut settings = Settings::new();
    settings.set(name, value)?;

    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    storage::setting::update_setting(&mut tx, name, value)?;

    tx.commit()
}

fn get_all_settings(db_path: &Path) -> Result<Settings> {
    let mut store = Storage::open(&db_path)?;
    let mut tx = store.begin_transaction()?;

    let settings = storage::setting::settings(&mut tx)?;

    tx.commit()?;

    Ok(settings)
}
