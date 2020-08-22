use std::path::Path;

use chrono::DateTime;

use crate::entities::{path::ScopedPath, File, FileId, FileSort};
use crate::errors::*;
use crate::query::{
    AndExpression, ComparisonExpression, Expression, NotExpression, Operator, OrExpression,
    TagExpression,
};
use crate::storage::{collation_for, path_to_sql, Row, SqlBuilder, Transaction};

const TIMESTAMP_FORMAT: &str = "%F %T%.f%:z";

pub fn file_count(tx: &mut Transaction) -> Result<u64> {
    tx.count_from_table("file")
}

pub fn file_by_path(tx: &mut Transaction, scoped_path: &ScopedPath) -> Result<Option<File>> {
    let sql = "
SELECT id, directory, name, fingerprint, mod_time, size, is_dir
FROM file
WHERE directory = ? AND name = ?";

    let (dir, name) = scoped_path.inner_as_dir_and_name();

    let params = rusqlite::params![path_to_sql(dir)?, path_to_sql(name)?];
    tx.query_single_params(sql, params, parse_file)
}

fn parse_file(row: Row) -> Result<File> {
    let mod_time_str: String = row.get(4)?;
    let mod_time = DateTime::parse_from_str(&mod_time_str, TIMESTAMP_FORMAT)?;

    Ok(File {
        id: row.get(0)?,
        dir: row.get(1)?,
        name: row.get(2)?,
        fingerprint: row.get(3)?,
        mod_time,
        size: row.get_usize(5)?,
        is_dir: row.get(6)?,
    })
}

pub fn delete_untagged_files(tx: &mut Transaction, file_ids: &[FileId]) -> Result<()> {
    let sql = "
DELETE FROM file
WHERE id = ?1
AND (SELECT count(1)
     FROM file_tag
     WHERE file_id = ?1) == 0";

    for file_id in file_ids {
        let params = rusqlite::params![file_id];
        tx.execute_params(sql, params)?;
    }

    Ok(())
}

pub(crate) fn files_for_query(
    tx: &mut Transaction,
    expression: Option<&Expression>,
    explicit_only: bool,
    ignore_case: bool,
    path: Option<&ScopedPath>,
    path_contains_root: bool,
    file_sort: Option<FileSort>,
) -> Result<Vec<File>> {
    let builder = build_query(
        expression,
        explicit_only,
        ignore_case,
        path,
        path_contains_root,
        file_sort,
    )?;

    tx.query_vec_params(&builder.sql(), builder.params(), parse_file)
}

fn build_query(
    expression: Option<&Expression>,
    explicit_only: bool,
    ignore_case: bool,
    path: Option<&ScopedPath>,
    path_contains_root: bool,
    file_sort: Option<FileSort>,
) -> Result<SqlBuilder<'static>> {
    let mut builder = SqlBuilder::new();

    builder.append_sql(
        "
SELECT id, directory, name, fingerprint, mod_time, size, is_dir
FROM file
WHERE",
    );
    if let Some(expr) = expression {
        build_query_branch(&mut builder, expr, explicit_only, ignore_case);
    } else {
        builder.append_sql("1 == 1");
    }
    if let Some(path) = path {
        build_path_clause(&mut builder, path, path_contains_root)?;
    }
    if let Some(sort_type) = file_sort {
        build_sort(&mut builder, sort_type);
    }

    Ok(builder)
}

fn build_query_branch(
    builder: &mut SqlBuilder,
    expression: &Expression,
    explicit_only: bool,
    ignore_case: bool,
) {
    match expression {
        Expression::Not(not_expr) => {
            build_not_query_branch(builder, not_expr, explicit_only, ignore_case)
        }
        Expression::And(and_expr) => {
            build_and_query_branch(builder, and_expr, explicit_only, ignore_case)
        }
        Expression::Or(or_expr) => {
            build_or_query_branch(builder, or_expr, explicit_only, ignore_case)
        }
        Expression::Tag(tag_expr) => {
            build_tag_query_branch(builder, tag_expr, explicit_only, ignore_case)
        }
        Expression::Comparison(comp_expr) => {
            build_comp_query_branch(builder, comp_expr, explicit_only, ignore_case)
        }
    };
}

fn build_not_query_branch(
    builder: &mut SqlBuilder,
    not_expr: &NotExpression,
    explicit_only: bool,
    ignore_case: bool,
) {
    builder.append_sql("NOT");
    build_query_branch(builder, &not_expr.operand, explicit_only, ignore_case);
}

fn build_and_query_branch(
    builder: &mut SqlBuilder,
    and_expr: &AndExpression,
    explicit_only: bool,
    ignore_case: bool,
) {
    build_query_branch(builder, &and_expr.left, explicit_only, ignore_case);
    builder.append_sql("AND");
    build_query_branch(builder, &and_expr.right, explicit_only, ignore_case);
}

fn build_or_query_branch(
    builder: &mut SqlBuilder,
    or_expr: &OrExpression,
    explicit_only: bool,
    ignore_case: bool,
) {
    builder.append_sql("(");
    build_query_branch(builder, &or_expr.left, explicit_only, ignore_case);
    builder.append_sql("OR");
    build_query_branch(builder, &or_expr.right, explicit_only, ignore_case);
    builder.append_sql(")");
}

fn build_tag_query_branch(
    builder: &mut SqlBuilder,
    tag_expr: &TagExpression,
    explicit_only: bool,
    ignore_case: bool,
) {
    let collation = collation_for(ignore_case);

    if explicit_only {
        builder.append_sql(
            "
id IN (SELECT file_id
       FROM file_tag
       WHERE tag_id = (SELECT id
                       FROM tag
                       WHERE name",
        );
        builder.append_sql(collation);
        builder.append_sql(" = ");
        builder.append_param(tag_expr.tag.clone());
        builder.append_sql(
            "
                      )
      )",
        );
    } else {
        builder.append_sql(
            "
id IN (SELECT file_id
       FROM file_tag
       INNER JOIN (WITH RECURSIVE working (tag_id, value_id) AS
                   (
                       SELECT id, 0
                       FROM tag
                       WHERE name",
        );
        builder.append_sql(collation);
        builder.append_sql(" = ");
        builder.append_param(tag_expr.tag.clone());
        builder.append_sql(
            "
                       UNION ALL
                       SELECT b.tag_id, b.value_id
                       FROM implication b, working
                       WHERE b.implied_tag_id = working.tag_id AND
                             (b.implied_value_id = working.value_id OR working.value_id = 0)
                   )
                   SELECT tag_id, value_id
                   FROM working
                  ) imps
       ON file_tag.tag_id = imps.tag_id
       AND (file_tag.value_id = imps.value_id OR imps.value_id = 0)
      )",
        );
    }
}

fn build_comp_query_branch(
    builder: &mut SqlBuilder,
    comp_expr: &ComparisonExpression,
    explicit_only: bool,
    ignore_case: bool,
) {
    let collation = collation_for(ignore_case);

    let value_term = match comp_expr.value.parse::<f64>() {
        Ok(_) => "CAST(v.name AS float)",
        Err(_) => "v.name",
    };

    let mut operator = match comp_expr.operator {
        Operator::Equal => "==",
        Operator::Different => "!=",
        Operator::LessThan => "<",
        Operator::LessThanOrEqual => "<=",
        Operator::MoreThan => ">",
        Operator::MoreThanOrEqual => ">=",
    };

    if operator == "!=" {
        // Reinterpret as otherwise it won't work for multiple values of same tag
        // TODO: explain the problem more clearly (e.g. with an example)
        operator = "==";
        builder.append_sql(" NOT ");
    }

    if explicit_only {
        // FIXME: this is similar to the Go implementation but it seems buggy,
        // since the operator is not even used.
        builder.append_sql(
            "
id IN (SELECT file_id
       FROM file_tag
       WHERE tag_id = (SELECT id
                       FROM tag
                       WHERE name",
        );
        builder.append_sql(collation);
        builder.append_sql(" = ");
        builder.append_param(comp_expr.tag.clone());
        builder.append_sql(
            "          )
         AND value_id = (SELECT id
                         FROM value
                         WHERE name",
        );
        builder.append_sql(collation);
        builder.append_sql(" = ");
        builder.append_param(comp_expr.value.clone());
        builder.append_sql(
            "           )
      )",
        );
    } else {
        builder.append_sql(
            "
id IN (WITH RECURSIVE impft (tag_id, value_id) AS
       (
           SELECT t.id, v.id
           FROM tag t, value v
           WHERE t.name",
        );
        builder.append_sql(collation);
        builder.append_sql(" = ");
        builder.append_param(comp_expr.tag.clone());
        builder.append_sql(" AND ");
        builder.append_sql(value_term);
        builder.append_sql(collation);
        builder.append_sql(" ");
        builder.append_sql(operator);
        builder.append_sql(" ");
        builder.append_param(comp_expr.value.clone());
        builder.append_sql(
            "
           UNION ALL
           SELECT b.tag_id, b.value_id
           FROM implication b, impft
           WHERE b.implied_tag_id = impft.tag_id AND
                 (b.implied_value_id = impft.value_id OR impft.value_id = 0)
       )

       SELECT file_id
       FROM file_tag
       INNER JOIN impft
       ON file_tag.tag_id = impft.tag_id AND
          file_tag.value_id = impft.value_id
      )",
        );
    }
}

fn build_path_clause(
    builder: &mut SqlBuilder,
    path: &ScopedPath,
    path_contains_root: bool,
) -> Result<()> {
    builder.append_sql("AND (");

    let inner_path = path.inner();

    if inner_path == Path::new(".") {
        builder.append_sql("directory NOT LIKE '/%'");
    } else {
        builder.append_sql("directory = ");
        builder.append_param(path_to_sql(inner_path)?);
        builder.append_sql(" OR directory LIKE ");
        builder.append_param(path_to_sql(inner_path.join("%"))?);

        if path_contains_root {
            builder.append_sql(" OR directory NOT LIKE '/%'");
        }
    }

    let parent_dir = path.parent();
    let file_name = path.file_name();
    if let (Some(dir), Some(name)) = (parent_dir, file_name) {
        builder.append_sql(" OR (directory = ");
        builder.append_param(path_to_sql(dir)?);
        builder.append_sql(" AND name = ");
        builder.append_param(path_to_sql(name)?);
        builder.append_sql(")");
    }

    builder.append_sql(")");

    Ok(())
}

fn build_sort(builder: &mut SqlBuilder, sort_type: FileSort) {
    match sort_type {
        FileSort::Id => builder.append_sql("ORDER BY id"),
        FileSort::Name => builder.append_sql("ORDER BY directory || '/' || name"),
        FileSort::Time => builder.append_sql("ORDER BY mod_time, directory || '/' || name"),
        FileSort::Size => builder.append_sql("ORDER BY size, directory || '/' || name"),
    }
}
