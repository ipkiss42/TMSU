use std::ffi::OsString;
use std::fs;
use std::ops;
use std::path::{Path, PathBuf};
use std::rc::Rc;

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

    pub fn to_abs_path(&self) -> AbsPath {
        AbsPath::from_unchecked(self.0.clone())
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

pub struct AbsPath(PathBuf);

impl AbsPath {
    pub fn from_unchecked(abs: PathBuf) -> Self {
        assert!(
            abs.is_absolute(),
            "Expected an absolute path, but got '{}'",
            abs.display()
        );
        Self { 0: abs }
    }

    pub fn from<P: AsRef<Path>>(path: P, base: &AbsPath) -> Self {
        let path = path.as_ref();
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            clean(base.0.clone().join(path))
        };
        Self { 0: abs }
    }

    fn rel_to(&self, base: &CanonicalPath) -> Option<&Path> {
        // Sanity check
        assert!(
            base.is_dir(),
            "Bug: expected the base to be a directory: '{}'",
            base.display()
        );

        self.0.strip_prefix(base).ok()
    }
}

// Make all the `Path` methods available on AbsPath
impl ops::Deref for AbsPath {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

fn clean(p: PathBuf) -> PathBuf {
    // FIXME TODO: do not rely on path_clean, because:
    // 1. It doesn't support Windows properly
    // 2. It works on strings, but not on paths
    // We could do something similar to https://doc.rust-lang.org/std/path/struct.Path.html#method.components
    let s =
        path_clean::clean(p.to_str().unwrap_or_else(|| {
            panic!("Bug: path cannot be converted to a string: {}", p.display())
        }));
    PathBuf::from(s)
}

fn canonicalize_or_clean(path: PathBuf) -> Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize()?)
    } else {
        Ok(clean(path))
    }
}

fn is_symlink(path: &Path) -> bool {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        return metadata.file_type().is_symlink();
    }
    false
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
pub struct ScopedPath {
    // TODO: the `base` field is not really used after construction
    base: Rc<CanonicalPath>,
    inner: PathBuf,
}

impl ScopedPath {
    /// Create a new `ScopedPath` from a base and path.
    ///
    /// The `base` must be an existing directory (the code will panic otherwise).
    ///
    /// The given `path` can be either relative or absolute. If relative, it is assumed to be
    /// relative to `base`, not to the current directory.
    pub fn new<P: AsRef<Path>>(base: Rc<CanonicalPath>, path: P) -> Result<Self> {
        assert!(base.is_dir(), "The base must be a directory");

        let path = path.as_ref().to_path_buf();

        let mut growing = if path.is_relative() {
            base.to_path_buf()
        } else {
            PathBuf::from("")
        };

        // Iterate through the path components and append them to the "growing" path.
        // We stop either when there is no more component to add or, more importantly, when the new
        // component is inside the base and is a symlink.
        let mut components = path.components();
        while let Some(part) = components.next() {
            let extended = growing.join(part);
            if extended.starts_with(&*base) && is_symlink(&extended) {
                // At this point we know that "growing" is canonical (or clean), since every
                // iteration of the while loop must have been through the "else" clause.
                // We also know that "part" cannot be "..", otherwise either "growing" is not
                // canonical" or "extended" is not a symlink. So it is safe to simply use
                // "extended" without further processing.
                growing = extended;
                break;
            } else {
                growing = canonicalize_or_clean(extended)?;
            }
        }

        // Append the remaining components without resolving links
        growing.push(components.as_path());

        // Get the relative part
        let abs_path = AbsPath::from(growing, &base.to_abs_path());
        let mut inner = match abs_path.rel_to(&*base) {
            Some(rel) => rel.to_path_buf(),
            None => abs_path.0.clone(),
        };

        // Special case
        if inner == PathBuf::from("") {
            inner = PathBuf::from(".");
        }

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
    use std::fs;

    const TESTS_ROOT: &'static str = "/tmp/tmsu-tests";

    fn create_base() -> Rc<CanonicalPath> {
        let p = Path::new(TESTS_ROOT);
        fs::create_dir_all(&p).unwrap();
        Rc::new(CanonicalPath::new(&p).unwrap())
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
        let base = Rc::new(CanonicalPath::new(&root).unwrap());

        /// Helper function to reduce boilerplate
        fn assert_scoped_path<P1, P2>(base: Rc<CanonicalPath>, path: P1, expected_inner: P2)
        where
            P1: AsRef<Path>,
            P2: AsRef<Path>,
        {
            let path = path.as_ref();
            // Create the represented path as a directory, because canonicalization requires
            // the paths to exist
            fs::create_dir_all(base.join(path)).unwrap();
            let scoped_path = ScopedPath::new(base, path).unwrap();
            assert_eq!(scoped_path.inner, expected_inner.as_ref());
        }

        // Inside the root: relative
        assert_scoped_path(base.clone(), "rel", "rel");
        assert_scoped_path(base.clone(), join!(&root, "foo/bar"), "foo/bar");
        // Outside the root: absolute
        assert_scoped_path(base.clone(), "../other", join!(TESTS_ROOT, "other"));
        assert_scoped_path(base.clone(), "foo/../../other", join!(TESTS_ROOT, "other"));
        assert_scoped_path(
            base.clone(),
            join!(TESTS_ROOT, "dir"),
            join!(TESTS_ROOT, "dir"),
        );

        // Path clean up
        assert_scoped_path(base.clone(), "./dummy1/.././dummy2/../", ".");
        assert_scoped_path(base.clone(), "../root/dummy/../", ".");

        // Symlinks
        let symlink_out = join!(TESTS_ROOT, "symlink-out");
        let symlink_in = join!(&root, "symlink-in");
        // 1) Outside the root (relative): resolved
        std::fs::create_dir_all(join!(&root, "other")).unwrap();
        create_symlink(&root, &symlink_out);
        assert_scoped_path(base.clone(), "../symlink-out/other/", "other");
        // 2) Outside the root (absolute): resolved
        create_symlink(&root, &symlink_out);
        assert_scoped_path(base.clone(), join!(TESTS_ROOT, "symlink-out/aa"), "aa");
        // 3) Inside the root (relative): not resolved
        std::fs::create_dir_all(join!(&root, "aa")).unwrap();
        create_symlink(join!(&root, "other"), &symlink_in);
        assert_scoped_path(base.clone(), "symlink-in/aa", "symlink-in/aa");
        // 4) Inside the root (absolute): resolved
        create_symlink(join!(&root, "other"), &symlink_in);
        assert_scoped_path(base.clone(), join!(&root, "symlink-in/aa"), "symlink-in/aa");
    }

    #[test]
    fn test_inner_as_dir_and_name() {
        fn assert_dir_name(inner: &str, expected_dir: &str, expected_name: &str) {
            let (base, name) = ScopedPath::new(create_base(), inner)
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
