use std::fmt;

use lazy_static::lazy_static;
use regex::Regex;

use crate::errors::*;

// Initialize the regular expression only once, and on demand
lazy_static! {
    // Valid chars are the union of the following Unicode classes:
    //  * Letter
    //  * Numeric
    //  * Punctuation
    //  * Symbol
    //  * Space
    // This expression is negated to match invalid characters
    static ref INVALID_CHARS: Regex = Regex::new(r"[^\pL\pN\pP\pS\s]").unwrap();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TagId(pub u32);

impl fmt::Display for TagId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(pub u32);

impl ValueId {
    pub fn assert_non_zero(&self, msg: &str) {
        assert!(self.0 != 0, "{}", msg);
    }
}

impl fmt::Display for ValueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub struct Tag {
    pub id: TagId,
    pub name: String,
}

pub struct Value {
    pub id: ValueId,
    pub name: String,
}

pub struct TagFileCount {
    pub id: TagId,
    pub name: String,
    pub file_count: u32,
}

pub fn validate_tag_name(name: &str) -> Result<()> {
    validate_name_helper("tag names", name)
}

pub fn validate_value_name(name: &str) -> Result<()> {
    validate_name_helper("tag values", name)
}

/// Validate that the given name is a valid `what`.
/// This helper function is only there to avoid duplication, since tag names and tag values
/// currently have the same rules.
fn validate_name_helper(what: &str, name: &str) -> Result<()> {
    let error_message = match name {
        "" => Some("cannot be empty"),
        // Cannot be used in the VFS
        "." | ".." => Some("cannot be '.' or '..'"),
        // Used in the query language
        "and" | "AND" | "or" | "OR" | "not" | "NOT" => {
            Some("cannot be a logical operator: 'and', 'or' or 'not'")
        }
        // Used in the query language
        "eq" | "EQ" | "ne" | "NE" | "lt" | "LT" | "gt" | "GT" | "le" | "LE" | "ge" | "GE" => {
            Some("cannot be a comparison operator: 'eq', 'ne', 'gt', 'lt', 'ge' or 'le'")
        }
        _ => None,
    };
    if let Some(message) = error_message {
        return Err(format!("{} {}", what, message).into());
    }

    // Check Unicode characters
    if let Some(mat) = INVALID_CHARS.find(name) {
        // Unwrapping is safe because the regular expression always matches at least one character
        let bad_char = mat.as_str().chars().next().unwrap();
        let message = format!("{} cannot contain U+{:04X}", what, bad_char as u32);
        return Err(message.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_tag_name() {
        // "Normal" cases
        assert!(validate_tag_name("abc").is_ok());
        assert!(validate_tag_name("name='(1 or 2) lt 3'").is_ok());

        // Empty string
        assert!(validate_tag_name("").is_err());

        // Dots
        assert!(validate_tag_name(".").is_err());
        assert!(validate_tag_name("..").is_err());
        assert!(validate_tag_name("...").is_ok());
        assert!(validate_tag_name(".. ..").is_ok());

        // Logical and comparison operators: case matters
        assert!(validate_tag_name("and").is_err());
        assert!(validate_tag_name("AND").is_err());
        assert!(validate_tag_name("AnD").is_ok());
        assert!(validate_tag_name(" and ").is_ok());

        // Special characters and Unicode
        assert!(validate_tag_name(" \t\n\r ").is_ok());
        assert!(validate_tag_name("éüßżć").is_ok());
        assert!(validate_tag_name("今日は!").is_ok());
        assert!(validate_tag_name("control har").is_err());
    }

    #[test]
    fn test_validate_value_name() {
        // "Normal" cases
        assert!(validate_value_name("abc").is_ok());
        assert!(validate_value_name("name='(1 or 2) lt 3'").is_ok());

        // Empty string
        assert!(validate_value_name("").is_err());

        // Dots
        assert!(validate_value_name(".").is_err());
        assert!(validate_value_name("..").is_err());
        assert!(validate_value_name("...").is_ok());
        assert!(validate_value_name(".. ..").is_ok());

        // Logical and comparison operators: case matters
        assert!(validate_value_name("and").is_err());
        assert!(validate_value_name("AND").is_err());
        assert!(validate_value_name("AnD").is_ok());
        assert!(validate_value_name(" and ").is_ok());

        // Special characters and Unicode
        assert!(validate_value_name(" \t\n\r ").is_ok());
        assert!(validate_value_name("éüßżć").is_ok());
        assert!(validate_value_name("今日は!").is_ok());
        assert!(validate_value_name("control har").is_err());
    }
}
