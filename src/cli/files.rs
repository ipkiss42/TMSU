use std::env;
use std::path::PathBuf;

use clap::arg_enum;
use lazy_static::lazy_static;
use structopt::StructOpt;

use crate::api;
use crate::cli::{locate_db, GlobalOptions};
use crate::entities::FileSort;
use crate::errors::*;

lazy_static! {
    static ref EXAMPLES: String = super::generate_examples(&[
        (
            "tmsu files music mp3 # files with both 'music' and 'mp3'",
            None
        ),
        (
            "tmsu files music and mp3 # same query but with explicit 'and'",
            None
        ),
        ("tmsu files music and not mp3", None),
        ("tmsu files \"music and (mp3 or flac)\"", None),
        ("tmsu files \"year == 2017\"", None),
        ("tmsu files \"year < 2017\"", None),
        ("tmsu files year lt 2017", None),
        ("tmsu files year", None),
        ("tmsu files --path=/home/bob music", None),
        (r"tmsu files 'contains\=equals'", None),
        (r"tmsu files '\<tag\>'", None),
    ]);
}

arg_enum! {
    #[derive(Debug)]
    enum SortMode {
        Id,
        Name,
        Size,
        Time,
    }
}

/// Lists the files in the database that match the QUERY specified. If no query is specified, all
/// files in the database are listed.
///
/// QUERY may contain tag names to match, operators and parentheses.
/// Operators are: and or not == != < > <= >= eq ne lt gt le ge.
///
/// Queries are run against the database so the results may not reflect the current state of the
/// filesystem. Only tagged files are matched: to identify untagged files use the untagged
/// subcommand.
///
/// Note: If your tag or value name contains whitespace, operators (e.g. <) or parentheses (( or
/// )), these must be escaped with a backslash \, e.g. \<tag\> matches the tag name <tag>. Your
/// shell, however, may use some punctuation for its own purposes: this can normally be avoided by
/// enclosing the query in single quotation marks or by escaping the problem characters with a
/// backslash.
#[derive(Debug, StructOpt)]
#[structopt(after_help(EXAMPLES.as_str()))]
pub struct FilesOptions {
    /// Lists the number of files rather than their names
    #[structopt(short("c"), long("count"))]
    show_count: bool,

    /// Lists only items that are directories
    #[structopt(short, long("directory"))]
    directories_only: bool,

    /// Lists only items that are files
    #[structopt(short, long("file"))]
    files_only: bool,

    /// Sorts output. Possible values: id, none, name, size, time
    #[structopt(short, long)]
    sort: Option<SortMode>,

    /// Lists only explicitly tagged files
    #[structopt(short, long("explicit"))]
    explicit_only: bool,

    /// Ignores the case of tag and value names
    #[structopt(short, long)]
    ignore_case: bool,

    /// Delimits files with a NUL character rather than newline
    #[structopt(short("0"), long)]
    print0: bool,

    /// Lists only items under PATH
    #[structopt(name("path"), short, long)]
    base_path: Option<PathBuf>,

    /// Parts of the query
    #[structopt(name("query_part"))]
    query_parts: Vec<String>,
}

impl FilesOptions {
    pub fn execute(&self, global_opts: &GlobalOptions) -> Result<()> {
        let db_path = locate_db(&global_opts.database)?;
        info!("Database path: {}", db_path.display());

        // Base path sanity checks
        if let Some(path) = &self.base_path {
            if !path.exists() {
                return Err(
                    format!("invalid base path: no such directory '{}'", path.display()).into(),
                );
            }
            if !path.is_dir() {
                return Err(
                    format!("the base path must be a directory: '{}'", path.display()).into(),
                );
            }
        }

        let str_query = self.query_parts.join(" ");

        let files = api::files::list_matching(
            &db_path,
            &str_query,
            self.explicit_only,
            self.ignore_case,
            self.base_path.as_deref(),
            self.sort.as_ref().map(convert_sort_mode),
        )?;

        // Apply requested filters, if any
        let filtered_files: Vec<_> = files
            .iter()
            .filter(|fd| fd.is_dir || !self.directories_only)
            .filter(|fd| !fd.is_dir || !self.files_only)
            .collect();

        // Print matches
        if self.show_count {
            println!("{}", filtered_files.len());
        } else {
            let cwd = env::current_dir()?;
            for file_data in filtered_files {
                let rel_path = super::rel_to(&file_data.path, &cwd);
                if self.print0 {
                    print!("{}\0", rel_path.display());
                } else {
                    println!("{}", rel_path.display());
                }
            }
        }

        Ok(())
    }
}

fn convert_sort_mode(sort_mode: &SortMode) -> FileSort {
    match sort_mode {
        SortMode::Id => FileSort::Id,
        SortMode::Name => FileSort::Name,
        SortMode::Size => FileSort::Size,
        SortMode::Time => FileSort::Time,
    }
}
