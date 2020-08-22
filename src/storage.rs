mod schema;
mod upgrade;

use std::path::{Path, PathBuf};

use crate::errors::*;

pub struct Storage {
    pub db_path: PathBuf,
    pub root_path: PathBuf,
    conn: rusqlite::Connection,
}

impl Storage {
    pub fn create_at(db_path: &Path) -> Result<()> {
        info!("Creating database at {}", db_path.display());
        Self::create_or_open(db_path)?;
        Ok(())
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
            root_path: determine_root_path(&db_path)?,
            db_path,
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
    fn execute_params(&mut self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize> {
        Ok(self.tx.execute(sql, params)?)
    }

    fn query_single<T, F>(&mut self, sql: &str, f: F) -> Result<Option<T>>
    where
        F: FnOnce(Row<'_>) -> Result<T>,
    {
        let mut stmt = self.tx.prepare(sql)?;
        let mut rows = stmt.query(Self::NO_PARAMS)?;

        rows.next()?.map(|r| Row::new(r)).map(f).transpose()
    }
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
}
