use std::path::PathBuf;

use structopt::StructOpt;

use crate::api;
use crate::cli::{locate_db, GlobalOptions};
use crate::errors::*;

/// Lists or views the database settings for the current database.
///
/// Without arguments the complete set of settings are shown, otherwise lists the settings for the specified setting NAMEs.
///
/// If a VALUE is specified then the setting is updated.
#[derive(Debug, StructOpt)]
pub struct ConfigOptions {
    /// Config option name
    #[structopt(name = "setting")]
    settings: Vec<String>,
}

impl ConfigOptions {
    pub fn execute(&self, global_opts: &GlobalOptions) -> Result<()> {
        let db_path = locate_db(&global_opts.database)?;
        info!("Database path: {}", db_path.display());

        match self.settings.len() {
            0 => list_all_settings(&db_path)?,
            1 => process_param(&db_path, &self.settings[0], false)?,
            _ => {
                for setting in &self.settings {
                    process_param(&db_path, setting, true)?;
                }
            }
        }

        Ok(())
    }
}

fn list_all_settings(db_path: &PathBuf) -> Result<()> {
    let settings = api::config::run_config_list_all_settings(db_path)?;
    for setting in settings {
        println!("{}={}", setting.name, setting.value);
    }
    Ok(())
}

fn process_param(db_path: &PathBuf, setting_param: &str, print_with_name: bool) -> Result<()> {
    let parts: Vec<_> = setting_param.split('=').collect();
    match parts.len() {
        1 => print_single_setting(&db_path, setting_param, print_with_name)?,
        2 => amend_setting(&db_path, &parts[0], &parts[1])?,
        _ => return Err(format!("invalid argument, '{}'", setting_param).into()),
    }
    Ok(())
}

fn print_single_setting(
    db_path: &PathBuf,
    setting_name: &str,
    print_with_name: bool,
) -> Result<()> {
    let value = api::config::run_config_get_setting_value(db_path, setting_name)?;
    if print_with_name {
        println!("{}={}", setting_name, value);
    } else {
        println!("{}", value);
    }
    Ok(())
}

fn amend_setting(db_path: &PathBuf, setting: &str, new_value: &str) -> Result<()> {
    api::config::run_config_update_setting(db_path, setting, new_value)
}
