//! Error type shared by every module.

use std::fmt;

/// Everything that can go wrong while reading, selecting or rendering.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// A file or stdin could not be read.
    Io {
        source_name: String,
        message: String,
    },
    /// The CSV is malformed or lacks a required column.
    Schema {
        source_name: String,
        message: String,
    },
    /// One data row is invalid.
    Row {
        source_name: String,
        line: u64,
        message: String,
    },
    /// The input says two different things at once (two cost columns, a
    /// repeated key). Nothing is guessed; the exit code is 2.
    Ambiguous {
        source_name: String,
        /// The line of the row, or `None` for a problem in the header.
        line: Option<u64>,
        message: String,
    },
    /// The requested months or filters do not fit the data.
    Selection(String),
    /// The options contradict each other or the data.
    Usage(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io {
                source_name,
                message,
            } => write!(f, "cannot read {source_name}: {message}"),
            Error::Schema {
                source_name,
                message,
            } => write!(f, "{source_name}: {message}"),
            Error::Row {
                source_name,
                line,
                message,
            } => write!(f, "{source_name}, line {line}: {message}"),
            Error::Ambiguous {
                source_name,
                line: Some(line),
                message,
            } => write!(f, "{source_name}, line {line}: {message}"),
            Error::Ambiguous {
                source_name,
                line: None,
                message,
            } => write!(f, "{source_name}: {message}"),
            Error::Selection(message) | Error::Usage(message) => write!(f, "{message}"),
        }
    }
}

impl Error {
    /// The process exit code: 2 for ambiguous input, 1 for every other data error.
    pub fn exit_code(&self) -> u8 {
        match self {
            Error::Ambiguous { .. } => 2,
            _ => 1,
        }
    }
}

impl std::error::Error for Error {}

/// Why one field could not be read: it is invalid, or it says two things.
#[derive(Debug, PartialEq, Eq)]
pub enum Fault {
    Invalid(String),
    Ambiguous(String),
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::Invalid(m) | Fault::Ambiguous(m) => f.write_str(m),
        }
    }
}

impl From<String> for Fault {
    fn from(m: String) -> Fault {
        Fault::Invalid(m)
    }
}

impl From<&str> for Fault {
    fn from(m: &str) -> Fault {
        Fault::Invalid(m.to_string())
    }
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_io() {
        let e = Error::Io {
            source_name: "a.csv".into(),
            message: "no such file".into(),
        };
        assert_eq!(e.to_string(), "cannot read a.csv: no such file");
    }

    #[test]
    fn display_row_has_file_and_line() {
        let e = Error::Row {
            source_name: "a.csv".into(),
            line: 7,
            message: "bad cost".into(),
        };
        assert_eq!(e.to_string(), "a.csv, line 7: bad cost");
    }

    #[test]
    fn ambiguous_has_exit_code_two_and_names_the_line() {
        let e = Error::Ambiguous {
            source_name: "a.csv".into(),
            line: Some(3),
            message: "twice".into(),
        };
        assert_eq!(e.to_string(), "a.csv, line 3: twice");
        assert_eq!(e.exit_code(), 2);
        let h = Error::Ambiguous {
            source_name: "a.csv".into(),
            line: None,
            message: "twice".into(),
        };
        assert_eq!(h.to_string(), "a.csv: twice");
        assert_eq!(Error::Usage("u".into()).exit_code(), 1);
    }

    #[test]
    fn display_schema_and_selection() {
        let s = Error::Schema {
            source_name: "x".into(),
            message: "no cost column".into(),
        };
        assert_eq!(s.to_string(), "x: no cost column");
        assert_eq!(Error::Selection("m".into()).to_string(), "m");
        assert_eq!(Error::Usage("u".into()).to_string(), "u");
    }
}
