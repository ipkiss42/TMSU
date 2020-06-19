use lazy_static::lazy_static;
use structopt::StructOpt;

use crate::api;
use crate::cli::{extract_names, locate_db, print_columns, GlobalOptions, TagOrValueName};
use crate::errors::*;

lazy_static! {
    static ref EXAMPLES: String = super::generate_examples(&[
        ("tmsu values year", Some("2000\n2001\n2017")),
        ("tmsu values", Some("2000\n2001\n2017\ncheese\nopera")),
        ("tmsu values --count year", Some("3")),
    ]);
}

/// Lists the values for TAGs. If no TAG is specified then all tags are listed.
#[derive(Debug, StructOpt)]
#[structopt(after_help(EXAMPLES.as_str()))]
pub struct ValuesOptions {
    /// Lists the number of values rather than their names
    #[structopt(short("c"), long("count"))]
    show_count: bool,

    /// Lists one value per line
    #[structopt(short = "1")]
    one_per_line: bool,

    /// Tag names
    #[structopt()]
    names: Vec<TagOrValueName>,
}

impl ValuesOptions {
    pub fn execute(&self, global_opts: &GlobalOptions) -> Result<()> {
        let db_path = locate_db(&global_opts.database)?;
        info!("Database path: {}", db_path.display());

        let names = extract_names(&self.names);

        let values_output = api::values::run_values(&db_path, &names)?;

        match values_output.value_groups.len() {
            // When there is only one group, it means either that no tag was requested or that one
            // tag was requested. In either case, we don't print the tag name.
            // XXX: However, to mimic the Go implementation we escape values only when a tag was
            // requested.
            1 => print_group(
                None,
                &values_output.value_groups[0].value_names,
                self.show_count,
                self.one_per_line,
                !self.names.is_empty(),
            ),
            _ => {
                for value_group in values_output.value_groups {
                    print_group(
                        value_group.tag_name,
                        &value_group.value_names,
                        self.show_count,
                        self.one_per_line,
                        false,
                    );
                    if !self.show_count && self.one_per_line {
                        println!();
                    }
                }
            }
        }
        Ok(())
    }
}

fn print_group(
    tag_name: Option<String>,
    value_names: &[String],
    show_count: bool,
    one_per_line: bool,
    value_escaping: bool,
) {
    if show_count {
        match tag_name {
            Some(name) => println!("{}: {}", name, value_names.len()),
            None => println!("{}", value_names.len()),
        }
    } else {
        let mut escaped_values = Vec::from(value_names);
        if value_escaping {
            escaped_values = escaped_values
                .into_iter()
                .map(|n| escape_value(&n))
                .collect();
        }
        if one_per_line {
            if let Some(name) = tag_name {
                println!("{}", name);
            }
            for value_name in escaped_values {
                println!("{}", value_name);
            }
        } else {
            match tag_name {
                Some(name) => println!("{}: {}", name, escaped_values.join(" ")),
                None => print_columns(&escaped_values),
            }
        }
    }
}

fn escape_value(name: &str) -> String {
    super::escape(name.to_string(), &['=', ' '])
}
