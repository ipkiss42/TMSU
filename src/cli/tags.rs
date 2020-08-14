use std::path::PathBuf;

use ansi_term::{Colour, Style};
use clap::arg_enum;
use lazy_static::lazy_static;
use structopt::StructOpt;

use crate::api;
use crate::cli::{extract_names, locate_db, print_columns, GlobalOptions, TagOrValueName};
use crate::errors::*;

lazy_static! {
    static ref EXAMPLES: String = super::generate_examples(&[
        ("tmsu tags", Some("mp3 music opera")),
        ("tmsu tags tralala.mp3", Some("mp3 music opera")),
        (
            "tmsu tags tralala.mp3 boom.mp3",
            Some("./tralala.mp3: mp3 music opera\n./boom.mp3: mp3 music drum-n-bass")
        ),
        ("tmsu tags --count tralala.mp3", None),
        ("tmsu tags --value 2009 red", None),
    ]);
}

arg_enum! {
    #[derive(Debug, PartialEq)]
    enum PrintNameMode {
        Auto,
        Always,
        Never,
    }
}

/// Lists the tags applied to FILEs. If no FILE is specified then all tags in the database are listed.
///
/// When color is turned on, tags are shown in the following colors:
///
///   Normal An explicitly applied (regular) tag
///   Cyan   Tag implied by other tags
///   Yellow Tag is both explicitly applied and implied by other tags
///
/// See the imply subcommand for more information on implied tags.
#[derive(Debug, StructOpt)]
#[structopt(after_help(EXAMPLES.as_str()))]
pub struct TagsOptions {
    /// Lists the number of tags rather than their names
    #[structopt(short("c"), long("count"))]
    show_count: bool,

    /// Lists one tag per line
    #[structopt(short("1"))]
    one_per_line: bool,

    /// Do not show implied tags
    #[structopt(short, long)]
    explicit: bool,

    /// When to print the file/value name: auto, always, never
    #[structopt(short("n"), long("name"), default_value("auto"))]
    name_mode: PrintNameMode,

    /// Do not follow symlinks (show tags for symlink itself)
    #[structopt(short("P"), long)]
    no_dereference: bool,

    /// Show which tags utilize values
    #[structopt(short("u"), long("value"))]
    value_names: Vec<TagOrValueName>,

    /// File paths
    #[structopt(conflicts_with("values"))]
    paths: Vec<PathBuf>,
}

impl TagsOptions {
    pub fn execute(&self, global_opts: &GlobalOptions) -> Result<()> {
        let db_path = locate_db(&global_opts.database)?;
        info!("Database path: {}", db_path.display());

        let use_colors = super::should_use_colour(&global_opts.color);

        if !self.value_names.is_empty() {
            let value_names = extract_names(&self.value_names);
            let tag_groups = api::tags::list_tags_for_values(&db_path, &value_names)?;
            print_value_tag_groups(
                &tag_groups,
                &self.name_mode,
                self.show_count,
                self.one_per_line,
            );
        } else if !self.paths.is_empty() {
            let tag_groups =
                api::tags::list_tags_for_paths(&db_path, &self.paths, self.no_dereference)?;
            print_file_tag_groups(
                &tag_groups,
                &self.name_mode,
                self.show_count,
                self.one_per_line,
                use_colors,
            );
        } else {
            let tag_groups = api::tags::list_all_tags(&db_path)?;
            print_value_tag_groups(
                &tag_groups,
                &self.name_mode,
                self.show_count,
                self.one_per_line,
            );
        }

        Ok(())
    }
}

fn print_value_tag_groups(
    groups: &[api::tags::ValueTagGroup],
    name_mode: &PrintNameMode,
    show_count: bool,
    one_per_line: bool,
) {
    if groups.is_empty() {
        return;
    }

    // Decide whether to print the value name (if available)
    let print_value = name_mode != &PrintNameMode::Never && (
            name_mode == &PrintNameMode::Always ||
            groups.len() > 1 ||
            !super::is_stdout_tty()
        );

    match groups.len() {
        1 => {
            let value_name_opt = match print_value {
                true => &groups[0].value_name,
                false => &None,
            };

            print_value_tag_group(
                value_name_opt,
                &groups[0].tag_names,
                show_count,
                one_per_line,
            );
        }
        _ => {
            for tag_group in groups {
                let value_name_opt = match print_value {
                    true => &tag_group.value_name,
                    false => &None,
                };

                print_value_tag_group(
                    value_name_opt,
                    &tag_group.tag_names,
                    show_count,
                    one_per_line,
                );
                if !show_count && one_per_line {
                    println!();
                }
            }
        }
    };
}

fn print_value_tag_group(
    value_name: &Option<String>,
    tag_names: &[String],
    show_count: bool,
    one_per_line: bool,
) {
    if show_count {
        match value_name {
            Some(name) => println!("{}: {}", name, tag_names.len()),
            None => println!("{}", tag_names.len()),
        }
    } else if one_per_line {
        if let Some(name) = value_name {
            println!("{}", name);
        }
        for tag_name in tag_names {
            println!("{}", tag_name);
        }
    } else {
        match value_name {
            Some(name) => println!("{}: {}", name, tag_names.join(" ")),
            None => print_columns(tag_names),
        }
    }
}

fn print_file_tag_groups(
    groups: &[api::tags::FileTagGroup],
    name_mode: &PrintNameMode,
    show_count: bool,
    one_per_line: bool,
    use_colors: bool,
) {
    if groups.is_empty() {
        return;
    }

    // Decide whether to print the value name (if available)
    let print_path = name_mode != &PrintNameMode::Never && (
            name_mode == &PrintNameMode::Always ||
            groups.len() > 1 ||
            !super::is_stdout_tty()
        );

    match groups.len() {
        1 => {
            let path_opt = match print_path {
                true => Some(&groups[0].path),
                false => None,
            };

            print_file_tag_group(
                path_opt,
                &groups[0].tags,
                show_count,
                one_per_line,
                use_colors,
            );
        }
        _ => {
            for tag_group in groups {
                let path_opt = match print_path {
                    true => Some(&tag_group.path),
                    false => None,
                };

                print_file_tag_group(
                    path_opt,
                    &tag_group.tags,
                    show_count,
                    one_per_line,
                    use_colors,
                );
                if !show_count && one_per_line {
                    println!();
                }
            }
        }
    };
}

fn print_file_tag_group(
    file_path: Option<&PathBuf>,
    tags: &[api::tags::TagData],
    show_count: bool,
    one_per_line: bool,
    use_colors: bool,
) {
    if show_count {
        match file_path {
            Some(path) => println!("{}: {}", path.display(), tags.len()),
            None => println!("{}", tags.len()),
        }
    } else if one_per_line {
        if let Some(path) = file_path {
            println!("{}", path.display());
        }
        for tag_data in tags {
            println!("{}", format_tag_data(&tag_data, use_colors));
        }
    } else {
        let formatted: Vec<_> = tags
            .iter()
            .map(|td| format_tag_data(td, use_colors))
            .collect();
        match file_path {
            Some(path) => println!("{}: {}", path.display(), formatted.join(" ")),
            None => print_columns(&formatted),
        };
    }
}

fn format_tag_data(tag_data: &api::tags::TagData, use_colors: bool) -> String {
    let style = if use_colors {
        match (tag_data.explicit, tag_data.implicit) {
            (true, true) => Colour::Yellow.normal(),
            (false, true) => Colour::Cyan.normal(),
            _ => Style::default(),
        }
    } else {
        Style::default()
    };

    match &tag_data.value_name {
        None => style.paint(&tag_data.tag_name).to_string(),
        Some(val_name) => format!("{}={}", style.paint(&tag_data.tag_name), style.paint(val_name)),
    }
}
