use error_chain::ensure;

use crate::errors::*;

pub trait Setting {
    fn name(&self) -> &str;
    fn as_bool(&self) -> bool;
    fn as_str(&self) -> String;
    fn set(&mut self, value: &str) -> Result<()>;
}

struct BooleanValue<'a> {
    name: &'a str,
    value: bool,
}

impl<'a> BooleanValue<'a> {
    fn new(name: &'a str, value: bool) -> Self {
        Self { name, value }
    }
}

impl<'a> Setting for BooleanValue<'a> {
    fn name(&self) -> &str {
        self.name
    }

    fn as_bool(&self) -> bool {
        self.value
    }

    fn as_str(&self) -> String {
        match self.value {
            true => "yes",
            false => "no",
        }
        .to_owned()
    }

    fn set(&mut self, value: &str) -> Result<()> {
        self.value = match value {
            "yes" | "Yes" | "YES" | "true" | "True" | "TRUE" => true,
            "no" | "No" | "NO" | "false" | "False" | "FALSE" => false,
            _ => return Err(format!("invalid boolean value: {}", value).into()),
        };
        Ok(())
    }
}

struct StringValue<'a> {
    name: &'a str,
    value: String,
}

impl<'a> StringValue<'a> {
    fn new(name: &'a str, value: &str) -> Self {
        Self {
            name,
            value: value.to_owned(),
        }
    }
}

impl<'a> Setting for StringValue<'a> {
    fn name(&self) -> &str {
        self.name
    }

    fn as_bool(&self) -> bool {
        panic!("Cannot convert a string value to boolean: {}", self.value);
    }

    fn as_str(&self) -> String {
        self.value.clone()
    }

    fn set(&mut self, value: &str) -> Result<()> {
        ensure!(
            value != "",
            format!("setting '{}' must be specified", self.name())
        );
        self.value = value.to_owned();
        Ok(())
    }
}

pub struct Settings {
    settings: Vec<Box<dyn Setting>>,
}

impl Settings {
    const AUTO_CREATE_TAGS: &'static str = "autoCreateTags";
    const AUTO_CREATE_VALUES: &'static str = "autoCreateValues";
    const DIRECTORY_FINGERPRINT_ALGORITHM: &'static str = "directoryFingerprintAlgorithm";
    const FILE_FINGERPRINT_ALGORITHM: &'static str = "fileFingerprintAlgorithm";
    const REPORT_DUPLICATES: &'static str = "reportDuplicates";
    const SYMLINK_FINGERPRINT_ALGORITHM: &'static str = "symlinkFingerprintAlgorithm";

    /// Create a Settings instance with default values for all the settings
    pub fn new() -> Self {
        let defaults: Vec<Box<dyn Setting>> = vec![
            Box::new(BooleanValue::new(Self::AUTO_CREATE_TAGS, true)),
            Box::new(BooleanValue::new(Self::AUTO_CREATE_VALUES, true)),
            Box::new(StringValue::new(
                Self::DIRECTORY_FINGERPRINT_ALGORITHM,
                "none",
            )),
            Box::new(StringValue::new(
                Self::FILE_FINGERPRINT_ALGORITHM,
                "dynamic:SHA256",
            )),
            Box::new(BooleanValue::new(Self::REPORT_DUPLICATES, true)),
            Box::new(StringValue::new(
                Self::SYMLINK_FINGERPRINT_ALGORITHM,
                "follow",
            )),
        ];
        Self { settings: defaults }
    }

    pub fn list(&self) -> &[Box<dyn Setting>] {
        &self.settings
    }

    pub fn get(&self, name: &str) -> Option<&dyn Setting> {
        for entry in &self.settings {
            if entry.name() == name {
                return Some(entry.as_ref());
            }
        }
        None
    }

    /// Private helper method
    fn get_mut(&mut self, name: &str) -> Option<&mut Box<dyn Setting>> {
        for entry in &mut self.settings {
            if entry.name() == name {
                return Some(entry);
            }
        }
        None
    }

    pub fn set(&mut self, name: &str, value: &str) -> Result<()> {
        match self.get_mut(name) {
            None => Err(format!("Unknown option: {}", name).into()),
            Some(setting) => setting.set(value),
        }
    }

    /* Hide these methods until they get used, to avoid "unused" warnings

        pub fn auto_create_tags(&self) -> bool {
            self.get(Self::AUTO_CREATE_TAGS).unwrap().as_bool()
        }

        pub fn auto_create_values(&self) -> bool {
            self.get(Self::AUTO_CREATE_VALUES).unwrap().as_bool()
        }

        pub fn directory_fingerprint_algorithm(&self) -> String {
            self.get(Self::DIRECTORY_FINGERPRINT_ALGORITHM).unwrap().as_str()
        }

        pub fn file_fingerprint_algorithm(&self) -> String {
            self.get(Self::FILE_FINGERPRINT_ALGORITHM).unwrap().as_str()
        }

        pub fn report_duplicates(&self) -> bool {
            self.get("Self::REPORT_DUPLICATES").unwrap().as_bool()
        }

        pub fn symlink_fingerprint_algorithm(&self) -> String {
            self.get(Self::SYMLINK_FINGERPRINT_ALGORITHM).unwrap().as_str()
        }
    */
}
