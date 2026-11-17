//! The single entry point to load a map description, whatever its source: a built-in level, a
//! string (v1 or TOML) or a file.
use std::fs;

use crate::core::levels;

use super::{MapSpec, ParseError, parse_toml, parse_v1};

impl MapSpec {
    /// Parse a map from its content: as TOML (v2) when the content is a TOML document, and as a
    /// plain-text v1 map string otherwise.
    ///
    /// # Errors
    /// Any [`ParseError`] raised by the TOML or v1 parser.
    ///
    /// @ai-generated
    pub fn parse(content: &str) -> Result<Self, ParseError> {
        match parse_toml(content) {
            Ok(spec) => Ok(spec),
            Err(ParseError::NotV2) => parse_v1(content),
            Err(other) => Err(other),
        }
    }

    /// The built-in level `level`, numbered from 1.
    ///
    /// # Errors
    /// [`ParseError::InvalidLevel`] when `level` is 0 or greater than the number of levels.
    ///
    /// @ai-generated
    pub fn level(level: usize) -> Result<Self, ParseError> {
        let content = level
            .checked_sub(1)
            .and_then(|index| levels::LEVELS.get(index))
            .ok_or(ParseError::InvalidLevel {
                asked: level,
                min: 1,
                max: levels::LEVELS.len(),
            })?;
        Self::parse(content)
    }

    /// Load a map from a file. A built-in level name (`lvl<n>` or `level<n>`, case-insensitive)
    /// is resolved to that level without touching the file system.
    ///
    /// # Errors
    /// [`ParseError::InvalidFileName`] when the file cannot be read, or any parsing error.
    ///
    /// @ai-generated
    pub fn from_file(file: &str) -> Result<Self, ParseError> {
        if let Some(content) = levels::get_level_str(file) {
            return Self::parse(content);
        }
        let content = fs::read_to_string(file).map_err(|_| ParseError::InvalidFileName {
            file_name: file.into(),
        })?;
        Self::parse(&content)
    }
}
