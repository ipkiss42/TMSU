mod init;

use std::path::PathBuf;
use std::process;
use std::str;

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
    Init(init::InitOptions),
}

/// CLI entry point, dispatching to subcommands
pub fn run() -> Result<()> {
    let opt = TmsuOptions::from_args();

    match opt.cmd {
        SubCommands::Init(init_opts) => init_opts.execute(),
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
