use std::ffi::OsString;
use std::ops;
use std::path::{Path, PathBuf};

use crate::errors::*;

/// Simple wrapper around PathBuf to enforce stronger typing.
///
/// At creation time, the file/directory is guaranteed to exist and its path to be canonical.
#[derive(Debug, Clone)]
pub struct CanonicalPath(PathBuf);

impl CanonicalPath {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            0: path.as_ref().canonicalize()?,
        })
    }
}

impl AsRef<Path> for CanonicalPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

// Make all the `Path` methods available on CanonicalPath
impl ops::Deref for CanonicalPath {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

/// From a logical perspective, a `ScopedPath` holds an absolute path. However, it does not
/// necessarily store it internally as such.
///
/// A `ScopedPath` knows about a `base` directory. If the logical path is within the `base`
/// directory (possibly after cleaning up and resolving symlinks), then it is stored as a relative
/// path (relative to `base`). Otherwise it is stored as an absolute, canonical path.
///
/// See the documentation of `new()` for more details.
#[derive(Debug)]
pub struct ScopedPath<'a> {
    // TODO: the `base` field is not really used after construction
    base: &'a CanonicalPath,
    inner: PathBuf,
}

impl<'a> ScopedPath<'a> {
    /// Create a new `ScopedPath` from a base and path.
    ///
    /// The `base` must be an existing directory (the code will panic otherwise).
    ///
    /// The given `path` can be either relative or absolute. If relative, it is assumed to be
    /// relative to `base`, not to the current directory.
    pub fn new<P: AsRef<Path>>(base: &'a CanonicalPath, path: P) -> Result<Self> {
        assert!(base.is_dir(), "The base must be a directory");

        let path = path.as_ref();

        // Clean the given path
        let path_str = path.to_str().ok_or_else(|| {
            Error::from(format!("Cannot parse UTF-8 for path: '{}'", path.display()))
        })?;
        let cleaned_str = path_clean::clean(path_str);

        // Convert to PathBuf
        let mut path_buf = PathBuf::from(&cleaned_str);

        // We don't allow relative paths which "escape" from the base directory. In such a case, we
        // convert it into an absolute path. It will still have a chance to become relative later
        // on, in case '../foo' is equivalent to '.'.
        if path_buf.starts_with("..") {
            path_buf = base.join(path_buf);
        }

        let inner = if path_buf.is_relative() {
            // Nothing else to do for a relative path
            path_buf
        } else {
            // An absolute path will first be made canonical, to make it more likely to match the
            // prefix
            let path_buf = path_buf.canonicalize()?;

            // Try to make it relative
            if path_buf.starts_with(base) {
                let mut tmp = path_buf.strip_prefix(base)?.to_path_buf();
                // Special case: an empty path is converted to "."
                if tmp == Path::new("") {
                    tmp = PathBuf::from(".");
                }
                tmp
            } else {
                // The path is not inside the base directory; keep it absolute
                path_buf
            }
        };

        Ok(ScopedPath { base, inner })
    }

    /// Extract and return the base (parent directory) and name from the "inner" portion (which
    /// may still be absolute).
    ///
    /// Note that `/` and `.` are handled in a special way: both base and name will contain the
    /// same value. This is done to keep compatibility with existing sqlite DBs.
    pub fn inner_as_dir_and_name(&self) -> (OsString, OsString) {
        let mut base = match self.inner.parent() {
            Some(dir) => dir,
            // `None` is possible only if the path terminates in a root or prefix
            // In such a case, we return the root itself (i.e. the full path)
            None => &self.inner,
        };

        // Special case for the current directory
        if base == Path::new("") {
            base = Path::new(".");
        }

        let name = match self.inner.file_name() {
            Some(n) => n,
            None => {
                // The only valid case for this situation is when we are at the root
                assert!(
                    !self.inner.ends_with(".."),
                    "Invalid ScopedPath state (this is a bug)"
                );
                self.inner.as_os_str()
            }
        };

        (base.as_os_str().to_owned(), name.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use std::fs;

    const TESTS_ROOT: &'static str = "/tmp/tmsu-tests";

    lazy_static! {
        static ref BASE: CanonicalPath = {
            let p = Path::new(TESTS_ROOT);
            fs::create_dir_all(&p).unwrap();
            CanonicalPath::new(&p).unwrap()
        };
    }

    // TODO: support Windows?
    /// Create (or re-create) a `dst` symlink pointing to `src`
    fn create_symlink<P1, P2>(src: P1, dst: P2)
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let (src, dst) = (src.as_ref(), dst.as_ref());

        // Remove the target if it exists
        let attr = fs::symlink_metadata(dst);
        if let Ok(metadata) = attr {
            if metadata.file_type().is_dir() {
                fs::remove_dir(dst).unwrap();
            } else {
                fs::remove_file(dst).unwrap();
            }
        }

        std::os::unix::fs::symlink(src, dst).unwrap();
    }

    /// Join several parts of a path into a single PathBuf
    /// Copied from https://stackoverflow.com/a/40567215/2292504
    macro_rules! join {
        ($base:expr, $($segment:expr),+) => {{
            let mut base: ::std::path::PathBuf = $base.into();
            $(
                base.push($segment);
            )*
            base
        }}
    }

    #[test]
    fn construct_scoped_path() {
        let root = join!(TESTS_ROOT, "root");
        fs::create_dir_all(&root).unwrap();
        let base = CanonicalPath::new(&root).unwrap();

        /// Helper function to reduce boilerplate
        fn assert_scoped_path<P1, P2>(base: &CanonicalPath, path: P1, expected_inner: P2)
        where
            P1: AsRef<Path>,
            P2: AsRef<Path>,
        {
            let path = path.as_ref();
            // Create the represented path as a directory, because canonicalization requires
            // the paths to exist
            fs::create_dir_all(base.join(path)).unwrap();
            let scoped_path = ScopedPath::new(&base, path).unwrap();
            assert_eq!(scoped_path.inner, expected_inner.as_ref());
        }

        // Inside the root: relative
        assert_scoped_path(&base, "rel", "rel");
        assert_scoped_path(&base, join!(&root, "foo/bar"), "foo/bar");
        // Outside the root: absolute
        assert_scoped_path(&base, "../other", join!(TESTS_ROOT, "other"));
        assert_scoped_path(&base, join!(TESTS_ROOT, "dir"), join!(TESTS_ROOT, "dir"));

        // Path clean up
        assert_scoped_path(&base, "./dummy1/.././dummy2/../", ".");
        assert_scoped_path(&base, "../root/dummy/../", ".");

        // Symlinks
        let symlink_out = join!(TESTS_ROOT, "symlink-out");
        let symlink_in = join!(&root, "symlink-in");
        // 1) Outside the root (relative): resolved
        create_symlink(&root, &symlink_out);
        assert_scoped_path(&base, "../symlink-out/other/", "other");
        // 2) Outside the root (absolute): resolved
        create_symlink(&root, &symlink_out);
        assert_scoped_path(&base, join!(TESTS_ROOT, "symlink-out/aa"), "aa");
        // 3) Inside the root (relative): not resolved
        create_symlink(join!(&root, "other"), &symlink_in);
        assert_scoped_path(&base, "symlink-in/aa", "symlink-in/aa");
        // 4) Inside the root (absolute): resolved
        create_symlink(join!(&root, "other"), &symlink_in);
        assert_scoped_path(&base, join!(&root, "symlink-in/aa"), "other/aa");

        // Failed creations
        assert!(ScopedPath::new(&base, "../missing/dir").is_err());
        assert!(ScopedPath::new(&base, join!(TESTS_ROOT, "missing/dir")).is_err());
    }

    #[test]
    fn test_inner_as_dir_and_name() {
        fn assert_dir_name(inner: &str, expected_dir: &str, expected_name: &str) {
            let (base, name) = ScopedPath::new(&BASE, inner)
                .unwrap()
                .inner_as_dir_and_name();
            assert_eq!(base, OsString::from(expected_dir));
            assert_eq!(name, OsString::from(expected_name));
        }

        // Relative paths
        assert_dir_name("foo/bar", "foo", "bar");
        assert_dir_name("foo/bar/baz", "foo/bar", "baz");
        assert_dir_name("foo/bar/baz/", "foo/bar", "baz");

        // Absolute paths
        fs::create_dir_all("/tmp/foo/bar/baz").unwrap();
        assert_dir_name("/tmp/foo/bar", "/tmp/foo", "bar");
        assert_dir_name("/tmp/foo/bar/baz", "/tmp/foo/bar", "baz");
        assert_dir_name("/tmp/foo/bar/baz/", "/tmp/foo/bar", "baz");

        // Special cases
        assert_dir_name(".", ".", ".");
        assert_dir_name("/", "/", "/");
    }
}
