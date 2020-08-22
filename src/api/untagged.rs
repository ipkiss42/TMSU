use std::path::{Path, PathBuf};

use crate::entities::path::ScopedPath;
use crate::errors::*;
use crate::storage::{self, Storage};

// Implementation note: instead of a callback, returning an iterator might be slightly more
// elegant. Unfortunately, Rust generators are currently not stable, and without them the
// implementation would be quite complex.
pub fn list_untagged_for_paths(
    db_path: &Path,
    paths: &[PathBuf],
    recursive: bool,
    follow_symlinks: bool,
    cb: &mut dyn FnMut(&Path),
) -> Result<()> {
    let mut store = Storage::open(&db_path)?;
    let root_path = store.root_path.clone();
    let mut tx = store.begin_transaction()?;

    // Clone the paths and store them in a stack
    // Contrarily to the Go implementation, this uses a stack instead of recursion. To keep similar
    // ordering of results, the iterator is reversed, both here and when adding items to the stack
    // for the recursive directory traversal.
    let mut paths: Vec<_> = paths.iter().rev().map(|p| p.to_path_buf()).collect();

    while let Some(path) = paths.pop() {
        info!("Resolving path '{}'", path.display());
        let path = if follow_symlinks {
            path.canonicalize()?
        } else {
            path.to_path_buf()
        };

        info!("Looking up file '{}'", path.display());
        let scoped_path = ScopedPath::new(root_path.clone(), &path)?;
        let file_opt = storage::file::file_by_path(&mut tx, &scoped_path)?;

        // If the path is not tagged, trigger the callback
        if file_opt.is_none() {
            cb(&path);
        }

        if recursive && path.is_dir() {
            // Reverse the default order of directory entries
            let mut entries = vec![];
            for entry in path.read_dir()? {
                entries.push(entry?.path());
            }
            entries.reverse();

            paths.extend_from_slice(&entries);
        }
    }

    tx.commit()?;

    Ok(())
}
