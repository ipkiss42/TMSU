use std::path::PathBuf;

use ansi_term::Colour;
use lazy_static::lazy_static;
use structopt::StructOpt;

use crate::api;
use crate::cli::{locate_db, GlobalOptions, TagAndValueNames};
use crate::errors::*;

lazy_static! {
    static ref EXAMPLES: String = super::generate_examples(&[
        ("tmsu imply mp3 music", None),
        ("tmsu imply", Some("mp3 -> music")),
        ("tmsu imply aubergine aka=eggplant", None),
        ("tmsu imply --delete mp3 music", None),
    ]);
}

/// Creates a tag implication such that any file tagged TAG will be implicitly tagged IMPL.
///
/// When run without arguments lists the set of tag implications.
///
/// Tag implications are applied at time of file query (not at time of tag application) therefore any changes to the implication rules will affect all further queries.
///
/// By default the tag subcommand will not explicitly apply tags that are already implied by the implication rules.
///
/// The tags subcommand can be used to identify which tags applied to a file are implied.
#[derive(Debug, StructOpt)]
#[structopt(after_help(EXAMPLES.as_str()))]
pub struct ImplyOptions {
    /// Deletes the tag implication
    #[structopt(short, long, requires_all(&["tag", "implied"]))]
    delete: bool,

    /// Source tag for the implication
    #[structopt(requires("implied"))]
    tag: Option<TagAndValueNames>,

    /// Target tag(s) for the implication
    #[structopt(requires("tag"))]
    implied: Vec<TagAndValueNames>,
}

impl ImplyOptions {
    pub fn execute(&self, global_opts: &GlobalOptions) -> Result<()> {
        let db_path = locate_db(&global_opts.database)?;
        info!("Database path: {}", db_path.display());

        let use_colors = super::should_use_colour(&global_opts.color);

        match &self.tag {
            None => list_implications(&db_path, use_colors),
            Some(src_tag) => {
                if self.delete {
                    delete_implications(&db_path, src_tag, &self.implied)
                } else {
                    add_implications(&db_path, &src_tag, &self.implied)
                }
            }
        }
    }
}

fn list_implications(db_path: &PathBuf, use_colors: bool) -> Result<()> {
    info!("Retrieving tag implications");

    let implications_output = api::imply::run_imply_list(db_path)?;

    let max_implying_width = implications_output
        .implications
        .iter()
        .map(display_width)
        .max()
        .unwrap_or_default();

    for implication in implications_output.implications {
        print_implication(&implication, max_implying_width, use_colors);
    }

    Ok(())
}

fn display_width(implication: &api::imply::Implication) -> usize {
    implication.implying.tag_name.len()
        + match &implication.implying.value_name {
            None => 0,
            Some(name) => 1 + name.len(),
        }
}

fn print_implication(imp: &api::imply::Implication, max_implying_width: usize, use_colors: bool) {
    let implying_str = format_tag_value(&imp.implying.tag_name, &imp.implying.value_name);
    let mut implied_str = format_tag_value(&imp.implied.tag_name, &imp.implied.value_name);
    if use_colors {
        implied_str = Colour::Cyan.paint(implied_str).to_string();
    }

    println!(
        "{:>width$} -> {}",
        &implying_str,
        &implied_str,
        width = max_implying_width
    );
}

fn format_tag_value(tag_name: &str, value_name: &Option<String>) -> String {
    match value_name {
        None => tag_name.to_owned(),
        Some(val) => format!("{}={}", tag_name, val),
    }
}

fn delete_implications(
    db_path: &PathBuf,
    src_tag_and_val: &TagAndValueNames,
    implied: &[TagAndValueNames],
) -> Result<()> {
    let implications = create_api_implications(&src_tag_and_val, implied);
    api::imply::delete_implications(db_path, &implications)
}

fn add_implications(
    db_path: &PathBuf,
    src_tag_and_val: &TagAndValueNames,
    implied: &[TagAndValueNames],
) -> Result<()> {
    let implications = create_api_implications(&src_tag_and_val, implied);
    api::imply::add_implications(db_path, &implications)
}

fn create_api_implications(
    implying: &TagAndValueNames,
    implied: &[TagAndValueNames],
) -> Vec<api::imply::Implication> {
    implied
        .iter()
        .map(|tgt| api::imply::TagAndOptionalValue {
            tag_name: tgt.tag_name.clone(),
            value_name: tgt.value_name.clone(),
        })
        .map(|tag_and_value| api::imply::Implication {
            implying: api::imply::TagAndOptionalValue {
                tag_name: implying.tag_name.clone(),
                value_name: implying.value_name.clone(),
            },
            implied: tag_and_value,
        })
        .collect()
}
