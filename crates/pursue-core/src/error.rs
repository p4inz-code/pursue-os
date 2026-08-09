//! The unified error type shared across PURSUE foundation crates.

use std::fmt;

/// The unified error type for PURSUE foundation crates.
///
/// `#[non_exhaustive]`: new variants may be added without breaking API users.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The caller supplied invalid input (e.g., an empty source label).
    InvalidInput(String),
    /// A string was not valid hexadecimal.
    InvalidHex(String),
    /// A string was not a valid content address.
    InvalidContentAddress(String),
    /// The requested item does not exist.
    NotFound(String),
    /// An integrity check failed; data was altered or corrupted.
    IntegrityViolation(String),
    /// An underlying I/O operation failed.
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Error::InvalidHex(msg) => write!(f, "invalid hex: {msg}"),
            Error::InvalidContentAddress(msg) => write!(f, "invalid content address: {msg}"),
            Error::NotFound(msg) => write!(f, "not found: {msg}"),
            Error::IntegrityViolation(msg) => write!(f, "integrity violation: {msg}"),
            Error::Io(err) => write!(f, "i/o error: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

/// Convenience alias for [`std::result::Result`] with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn error_displays_human_readably() {
        let err = Error::IntegrityViolation("hash mismatch".into());
        assert_eq!(err.to_string(), "integrity violation: hash mismatch");
    }

    #[test]
    fn io_error_converts_and_is_source() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let err: Error = io.into();
        assert!(matches!(err, Error::Io(_)));
        let source = std::error::Error::source(&err);
        assert!(source.is_some());
    }
}
