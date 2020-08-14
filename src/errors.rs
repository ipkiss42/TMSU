use std::path::PathBuf;

use error_chain::error_chain;

error_chain! {
    errors {
        NoDatabaseFound(path: PathBuf) {
            description("No database found")
            display("No database found at '{}'", path.display())
        }
        DatabaseAccessError(path: PathBuf) {
            description("Cannot open database")
            display("Cannot open database at '{}'", path.display())
        }
    }
    foreign_links {
        Chrono(chrono::ParseError);
        Io(std::io::Error);
        StripPrefix(std::path::StripPrefixError);
        Rusqlite(rusqlite::Error);
    }
}
