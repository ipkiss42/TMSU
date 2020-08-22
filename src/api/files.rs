use std::path::{Path, PathBuf};

use crate::entities;
use crate::entities::{path::CasedContains, path::ScopedPath, FileSort};
use crate::errors::*;
use crate::query::Expression;
use crate::storage::{self, Storage, Transaction};

pub struct FileData {
    pub path: PathBuf,
    pub is_dir: bool,
}

pub fn list_matching(
    db_path: &Path,
    str_query: &str,
    explicit_only: bool,
    ignore_case: bool,
    path: Option<&Path>,
    file_sort: Option<FileSort>,
) -> Result<Vec<FileData>> {
    let mut store = Storage::open(&db_path)?;

    let mut path_contains_root = false;
    if let Some(p) = path {
        path_contains_root = store.path_contains_root(p)?;
    }

    let root_path = store.root_path.clone();
    let mut tx = store.begin_transaction()?;

    info!("Parsing query");
    let expr_opt: Option<Expression> = Expression::parse(str_query)?;
    debug!("Parsed query: {:?}", expr_opt);

    // Sanity checks
    if let Some(ref expr) = expr_opt {
        check_tag_names(&mut tx, &expr, ignore_case)?;
        check_value_names(&mut tx, &expr, ignore_case)?;
    }

    info!("Querying database");
    let scoped_base_path = path
        .map(|p| ScopedPath::new(root_path.clone(), p))
        .transpose()?;
    // TODO: custom error message in case of too complex queries. See Go implementation:
    // https://github.com/oniony/TMSU/blob/1e6dab9fc21bb8d498af9e45aec10440500b8ea9/src/github.com/oniony/TMSU/cli/files.go#L159-L165
    let files = storage::file::files_for_query(
        &mut tx,
        expr_opt.as_ref(),
        explicit_only,
        ignore_case,
        scoped_base_path.as_ref(),
        path_contains_root,
        file_sort,
    )?;

    tx.commit()?;

    Ok(files
        .into_iter()
        .map(|f| FileData {
            path: f.to_abs_path(&*root_path),
            is_dir: f.is_dir,
        })
        .collect())
}

/// Check that the tag names in the given expression are valid and that they are present in the
/// database.
/// FIXME (or not?): the Go implementation was only emitting warnings, but here we fail hard at the
/// first problem.
fn check_tag_names(tx: &mut Transaction, expr: &Expression, ignore_case: bool) -> Result<()> {
    debug!("Checking tag names");

    let tag_names = expr.tag_names();
    let db_tag_names = storage::tag::tags_by_names(tx, &tag_names, ignore_case)?;

    for tag in tag_names {
        entities::validate_tag_name(tag)?;

        if !db_tag_names.contains_for_case(tag, ignore_case) {
            return Err(format!("no such tag '{}'", tag).into());
        }
    }

    Ok(())
}

/// Check that the tag names in the given expression are valid and that they are present in the
/// database.
/// FIXME (or not?): the Go implementation was only emitting warnings, but here we fail hard at the
/// first problem.
fn check_value_names(tx: &mut Transaction, expr: &Expression, ignore_case: bool) -> Result<()> {
    debug!("Checking value names");

    let value_names = expr.exact_value_names();
    let db_value_names = storage::value::values_by_names(tx, &value_names, ignore_case)?;

    for value in value_names {
        entities::validate_tag_name(value)?;

        if !db_value_names.contains_for_case(value, ignore_case) {
            return Err(format!("no such value '{}'", value).into());
        }
    }

    Ok(())
}
