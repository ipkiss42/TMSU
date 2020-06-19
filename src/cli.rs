mod delete;
mod info;
mod init;
mod merge;
mod rename;
mod values;

use std::env;
use std::path::PathBuf;
use std::process;
use std::result;
use std::str;
use std::string;

use ansi_term::Colour;
use clap::arg_enum;
use clap::AppSettings::{ColoredHelp, UnifiedHelpMessage};
use structopt::StructOpt;

use crate::errors::*;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "TMSU",
    about = "A tool for tagging your files and accessing them through a virtual filesystem",
    global_setting(UnifiedHelpMessage),  // Merge options and flags in the usage output
    global_setting(ColoredHelp),  // Use colors by default
)]
struct TmsuOptions {
    // Externalize global options to a separate struct for convenience
    #[structopt(flatten)]
    global_opts: GlobalOptions,

    #[structopt(subcommand)]
    cmd: SubCommands,
}

#[derive(Debug, StructOpt)]
pub struct GlobalOptions {
    /// Use the specified database
    #[structopt(short = "-D", long, env = "TMSU_DB", parse(from_os_str))]
    database: Option<PathBuf>,

    /// Colorize the output (auto/always/never)
    #[structopt(long, default_value = "auto")]
    color: ColorMode,
}

arg_enum! {
    #[derive(Debug)]
    enum ColorMode {
        Auto,
        Always,
        Never,
    }
}

#[derive(Debug, StructOpt)]
enum SubCommands {
    Delete(delete::DeleteOptions),
    Info(info::InfoOptions),
    Init(init::InitOptions),
    Merge(merge::MergeOptions),
    Rename(rename::RenameOptions),
    Values(values::ValuesOptions),
}

/// CLI entry point, dispatching to subcommands
pub fn run() -> Result<()> {
    let opt = TmsuOptions::from_args();

    match opt.cmd {
        SubCommands::Delete(delete_opts) => delete_opts.execute(&opt.global_opts),
        SubCommands::Info(info_opts) => info_opts.execute(&opt.global_opts),
        SubCommands::Init(init_opts) => init_opts.execute(),
        SubCommands::Merge(merge_opts) => merge_opts.execute(&opt.global_opts),
        SubCommands::Rename(rename_opts) => rename_opts.execute(&opt.global_opts),
        SubCommands::Values(values_opts) => values_opts.execute(&opt.global_opts),
    }
}

fn locate_db(db_path: &Option<PathBuf>) -> Result<PathBuf> {
    // Use the given path if available
    match db_path {
        Some(path) => Ok(path.clone()),
        // Fallback: look for the DB in parent directories
        None => match find_database_upwards()? {
            Some(path) => Ok(path),
            // Fallback: use the default database
            None => match get_user_default_db() {
                Some(path) => Ok(path),
                // OK, we finally give up...
                None => Err(ErrorKind::NoDatabaseFound(PathBuf::default()).into()),
            },
        },
    }
}

/// Look for .tmsu/db in the current directory and ancestors
fn find_database_upwards() -> Result<Option<PathBuf>> {
    let mut path = env::current_dir()?;

    loop {
        let mut db_path = path.clone();
        db_path.push(".tmsu");
        db_path.push("db");

        debug!("Looking for database at {:?}", &db_path);
        if db_path.is_file() {
            return Ok(Some(db_path));
        }

        match path.parent() {
            Some(parent) => {
                path = PathBuf::from(parent);
            }
            None => {
                return Ok(None);
            }
        }
    }
}

/// Return the path corresponding to $HOME/.tmsu/default.db,
/// or None if the home directory cannot be resolved
fn get_user_default_db() -> Option<PathBuf> {
    dirs::home_dir().map(|mut path| {
        path.push(".tmsu");
        path.push("default.db");
        path
    })
}

fn is_stdout_tty() -> bool {
    atty::is(atty::Stream::Stdout)
}

fn should_use_colour(color_mode: &ColorMode) -> bool {
    match color_mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => is_stdout_tty(),
    }
}

pub fn print_error(result: Result<()>) {
    if let Err(error) = result {
        eprintln!("tmsu: {}", error);

        if let Some(backtrace) = error.backtrace() {
            eprintln!("backtrace: {:?}", backtrace);
        }

        process::exit(1);
    }
}

fn generate_examples(examples: &[(&str, Option<&str>)]) -> String {
    // Simple indirection to make testing easier
    generate_examples_inner(is_stdout_tty(), examples)
}

fn generate_examples_inner(use_color: bool, examples: &[(&str, Option<&str>)]) -> String {
    // Define styles
    let header_style;
    let prompt_style;
    if use_color {
        header_style = Colour::Yellow.normal();
        prompt_style = Colour::Green.normal();
    } else {
        header_style = ansi_term::Style::default();
        prompt_style = ansi_term::Style::default();
    }

    let prompt = prompt_style.paint("$");

    let formatted: Vec<_> = examples
        .iter()
        .map(|(cmd_line, output)| {
            let output_str = match output {
                Some(s) => format!("\n    {}", s.replace("\n", "\n    ")),
                None => "".to_string(),
            };
            format!("    {} {}{}", prompt, cmd_line, output_str)
        })
        .collect();
    format!(
        "{}\n{}",
        header_style.paint("EXAMPLES:"),
        formatted.join("\n")
    )
}

fn print_columns(strings: &[String]) {
    // TODO: do proper column printing
    if is_stdout_tty() {
        println!("{}", strings.join("  "));
    } else {
        for s in strings {
            println!("{}", s);
        }
    }
}

#[derive(Debug)]
struct TagOrValueName {
    name: String,
}

// Allow automatic parsing in StructOpt
impl str::FromStr for TagOrValueName {
    type Err = string::ParseError;

    fn from_str(raw: &str) -> result::Result<Self, Self::Err> {
        let mut name = String::with_capacity(raw.len());

        let mut escaped = false;
        for chr in raw.chars() {
            if escaped {
                escaped = false;
                name.push(chr);
                continue;
            }

            match chr {
                '\\' => escaped = true,
                _ => name.push(chr),
            };
        }

        Ok(TagOrValueName { name })
    }
}

fn extract_names(parsed_names: &[TagOrValueName]) -> Vec<&str> {
    parsed_names.iter().map(|tovn| &tovn.name as &str).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn gen_examples() {
        // Single example without output
        assert_eq!(
            generate_examples_inner(false, &[("hello", None)]),
            "EXAMPLES:
    $ hello"
        );

        // Single example with multi-line output
        assert_eq!(
            generate_examples_inner(
                false,
                &[("command", Some("this is some\nmulti-line output"))]
            ),
            "EXAMPLES:
    $ command
    this is some
    multi-line output"
        );

        // Mixing examples with and without output
        assert_eq!(
            generate_examples_inner(
                false,
                &[
                    ("mkdir tmp-dir", None),
                    ("cd tmp-dir", None),
                    ("tmsu init", Some("tmsu: /tmp/tmp-dir: creating database"))
                ]
            ),
            "EXAMPLES:
    $ mkdir tmp-dir
    $ cd tmp-dir
    $ tmsu init
    tmsu: /tmp/tmp-dir: creating database"
        );

        // With colors
        assert_eq!(
            generate_examples_inner(true, &[("hello", None)]),
            "\u{1b}[33mEXAMPLES:\u{1b}[0m
    \u{1b}[32m$\u{1b}[0m hello"
        );
    }

    #[test]
    fn parse_tag_or_value_name() {
        // Helper function to remove some boilerplate
        fn assert_parse(raw: &str, expected_tag_name: &str) {
            let parsed = TagOrValueName::from_str(raw).unwrap();
            assert_eq!(parsed.name, expected_tag_name);
        }

        assert_parse(r"abc", "abc");
        assert_parse(r"a or (b == c)", "a or (b == c)");
        assert_parse(r"\\\a\ or \(b =\= c)", r"\a or (b == c)");
        // TODO: similar to Go, but should perhaps be disallowed
        assert_parse(r"trailing\", "trailing");
    }
}
