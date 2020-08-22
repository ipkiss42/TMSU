pub mod file;
pub mod filetag;
pub mod implication;
mod schema;
pub mod setting;
pub mod tag;
mod upgrade;
pub mod value;

use std::iter;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::entities::path::{CanonicalPath, ScopedPath};
use crate::entities::{FileId, TagId, ValueId};
use crate::errors::*;

pub struct Storage {
    pub db_path: CanonicalPath,
    // The root path is stored as a Rc, because it is immutable and will likely be shared with many
    // ScopedPath instances. These instances cannot use a simple reference (i.e. they need shared
    // ownership), because they can outlive the "api" layer where the Storage is created.
    pub root_path: Rc<CanonicalPath>,
    conn: rusqlite::Connection,
}

impl Storage {
    pub fn create_at(db_path: &Path) -> Result<()> {
        info!("Creating database at {}", db_path.display());
        Self::create_or_open(db_path)?;
        Ok(())
    }

    pub fn open(db_path: &Path) -> Result<Self> {
        info!("Opening database at {}", db_path.display());
        Self::create_or_open(db_path)
    }

    /// Open a sqlite3 DB file, also creating it if it doesn't already exist.
    /// Note that the parent directory will NOT be created if it doesn't exist.
    fn create_or_open(db_path: &Path) -> Result<Self> {
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|_| ErrorKind::DatabaseAccessError(db_path.to_path_buf()))?;

        // Use a canonical path to avoid issues such as #168
        let db_path = db_path
            .canonicalize()
            .map_err(|_| ErrorKind::NoDatabaseFound(db_path.to_path_buf()))?;

        let mut res = Storage {
            root_path: Rc::new(CanonicalPath::new(determine_root_path(&db_path)?)?),
            db_path: CanonicalPath::new(db_path)?,
            conn,
        };

        res.upgrade_database()?;

        Ok(res)
    }

    pub fn begin_transaction<'a>(&'a mut self) -> Result<Transaction<'a>> {
        Ok(Transaction {
            tx: self.conn.transaction()?,
        })
    }

    fn upgrade_database(&mut self) -> Result<()> {
        let mut tx = self.begin_transaction()?;

        upgrade::upgrade(&mut tx)?;

        tx.commit()?;
        Ok(())
    }

    /// Return true iff the given path is a parent of (or identical to) the Store root
    pub fn path_contains_root<P: AsRef<Path>>(&self, path: P) -> Result<bool> {
        // Much simpler implementation than in Go, since we can leverage ScopedPath
        // to do all the hard work (by inverting the usual base and path)
        let canonical = CanonicalPath::new(path)?;
        let scoped = ScopedPath::new(Rc::new(canonical), self.root_path.as_ref())?;
        Ok(scoped.inner().is_relative())
    }
}

fn determine_root_path(db_path: &Path) -> Result<PathBuf> {
    let parent_opt = db_path.parent();
    let name_opt = parent_opt.map(|p| p.file_name()).flatten();
    if let Some(dir_name) = name_opt {
        // If a directory has a name, parent_opt cannot be None
        let parent = parent_opt.unwrap();

        if dir_name == ".tmsu" {
            // The unwrap() call should never fail for a canonical path
            Ok(parent.parent().unwrap().to_path_buf())
        } else {
            // Unexpected directory name: return the direct parent
            // Note that this differs from the Go implementation
            Ok(parent.to_path_buf())
        }
    } else {
        Err("Could not determine root path".into())
    }
}

pub struct Transaction<'a> {
    tx: rusqlite::Transaction<'a>,
}

// This implementation exposes useful methods from the underlying DB transaction.
// Note that more work would be needed for an encapsulation which doesn't leak rusqlite structs
// (e.g. Statement or ToSql).
impl<'a> Transaction<'a> {
    pub fn commit(self) -> Result<()> {
        Ok(self.tx.commit()?)
    }

    // The helper functions below are not public, to be usable only from submodules.
    // They hide rusqlite-specific types (except for query params).

    const NO_PARAMS: &'a [&'a dyn rusqlite::ToSql] = rusqlite::NO_PARAMS;

    /// Execute a SQL statement taking no parameter
    fn execute(&mut self, sql: &str) -> Result<usize> {
        Ok(self.tx.execute(sql, Self::NO_PARAMS)?)
    }

    /// Execute a SQL statement taking unnamed parameters
    fn execute_params<P>(&mut self, sql: &str, params: P) -> Result<usize>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
    {
        Ok(self.tx.execute(sql, params)?)
    }

    /// Execute a query and create one object per returned line.
    ///
    /// This is similar to rusqlite::Statement::query_map_and_then(), but the passed function can
    /// return errors that are not from rusqlite.
    fn query_vec<T, F>(&mut self, sql: &str, f: F) -> Result<Vec<T>>
    where
        F: Fn(Row<'_>) -> Result<T>,
    {
        self.query_vec_params(sql, Self::NO_PARAMS, f)
    }

    fn query_vec_params<T, P, F>(&mut self, sql: &str, params: P, f: F) -> Result<Vec<T>>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
        F: Fn(Row<'_>) -> Result<T>,
    {
        let mut stmt = self.tx.prepare(sql)?;
        let mut rows = stmt.query(params)?;

        let mut objects = Vec::new();
        while let Some(row) = rows.next()? {
            objects.push(f(Row::new(row))?);
        }

        Ok(objects)
    }

    fn query_single<T, F>(&mut self, sql: &str, f: F) -> Result<Option<T>>
    where
        F: FnOnce(Row<'_>) -> Result<T>,
    {
        self.query_single_params(sql, Self::NO_PARAMS, f)
    }

    fn query_single_params<T, P, F>(&mut self, sql: &str, params: P, f: F) -> Result<Option<T>>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
        F: FnOnce(Row<'_>) -> Result<T>,
    {
        let mut stmt = self.tx.prepare(sql)?;
        let mut rows = stmt.query(params)?;

        rows.next()?.map(|r| Row::new(r)).map(f).transpose()
    }

    fn count_from_table(&mut self, table_name: &str) -> Result<u64> {
        let sql = format!(
            "
SELECT count(*)
FROM {}",
            table_name
        );

        let value: u32 = self.tx.query_row(&sql, Self::NO_PARAMS, |row| row.get(0))?;
        Ok(value as u64)
    }

    fn last_inserted_row_id(&mut self) -> u32 {
        self.tx.last_insert_rowid() as u32
    }
}

/// Generate a string such as "?,?,?", with as many placeholders ('?') as requested
fn generate_placeholders<'a>(values: &'a [&str]) -> Result<(String, Vec<&'a dyn rusqlite::ToSql>)> {
    error_chain::ensure!(!values.is_empty(), "Bug: expected at least one placeholder");
    let placeholders: Vec<_> = iter::repeat("?").take(values.len()).collect();
    placeholders.join(",");

    let mut params = Vec::with_capacity(values.len());
    for value in values {
        params.push(value as &dyn rusqlite::ToSql);
    }

    Ok((placeholders.join(","), params))
}

/// Convert a path-like object into a string. Note that this conversion can fail.
/// TODO: does this really work on Windows? If not, what to do instead?
fn path_to_sql<'a, P: 'a + AsRef<Path>>(path: P) -> Result<String> {
    Ok(path
        .as_ref()
        .to_str()
        .ok_or_else(|| Error::from("Cannot convert to str"))?
        .to_string())
}

/// Simple wrapper around a rusqlite::Row, mostly to avoid explicit error conversions in callbacks.
/// It's not clear whether this is really worth it...
struct Row<'a>(&'a rusqlite::Row<'a>);

impl<'a> Row<'a> {
    fn new(row: &'a rusqlite::Row<'a>) -> Self {
        Self { 0: row }
    }

    fn column_count(&self) -> usize {
        self.0.column_count()
    }

    fn get<I, T>(&self, index: I) -> Result<T>
    where
        I: rusqlite::RowIndex,
        T: rusqlite::types::FromSql,
    {
        Ok(self.0.get(index)?)
    }

    fn get_usize<I: rusqlite::RowIndex>(&self, index: I) -> Result<usize> {
        let tmp: i64 = self.0.get(index)?;
        // Force cast to usize: we don't expect negative values
        Ok(tmp as usize)
    }
}

impl rusqlite::types::FromSql for TagId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        u32::column_result(value).map(TagId)
    }
}

impl rusqlite::ToSql for TagId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl rusqlite::types::FromSql for ValueId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        u32::column_result(value).map(ValueId)
    }
}

impl rusqlite::ToSql for ValueId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl rusqlite::types::FromSql for FileId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        u32::column_result(value).map(FileId)
    }
}

impl rusqlite::ToSql for FileId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

type BoxedToSql = Box<dyn rusqlite::ToSql>;

struct SqlBuilder<'a> {
    sql_parts: Vec<&'a str>,
    params: Vec<BoxedToSql>,
    needs_param_comma: bool,
}

impl<'a> SqlBuilder<'a> {
    pub fn new() -> Self {
        Self {
            sql_parts: Vec::new(),
            params: Vec::new(),
            needs_param_comma: false,
        }
    }

    pub fn append_sql(&mut self, sql: &'a str) {
        match sql.chars().next() {
            None => return, // Empty string
            Some(chr) => match chr {
                ' ' | '\n' => (),
                _ => self.sql_parts.push("\n"),
            },
        };

        self.sql_parts.push(sql);

        self.needs_param_comma = false;
    }

    pub fn append_param(&mut self, param: impl rusqlite::ToSql + 'static) {
        if self.needs_param_comma {
            self.sql_parts.push(",");
        }

        self.sql_parts.push("?");

        self.params.push(Box::new(param));
        self.needs_param_comma = true;
    }

    pub fn sql(&self) -> String {
        self.sql_parts.concat()
    }

    pub fn params(self) -> impl IntoIterator<Item = BoxedToSql> {
        self.params
    }
}

fn collation_for(ignore_case: bool) -> &'static str {
    match ignore_case {
        true => " COLLATE NOCASE",
        false => "",
    }
}
