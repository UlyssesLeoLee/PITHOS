use std::fmt;

#[derive(Debug)]
pub enum DbError {
    Io(std::io::Error),
    Invalid(String),
    Corrupt(String),
    Busy(String),
    Missing(String),
    Incompatible(String),
    Unsupported(String),
    Plugin(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Invalid(s) => write!(f, "Invalid input: {s}"),
            Self::Corrupt(s) => write!(f, "Corrupt data: {s}"),
            Self::Busy(s) => write!(f, "Busy: {s}"),
            Self::Missing(s) => write!(f, "Not found: {s}"),
            Self::Incompatible(s) => write!(f, "Incompatible: {s}"),
            Self::Unsupported(s) => write!(f, "Unsupported: {s}"),
            Self::Plugin(s) => write!(f, "Plugin error: {s}"),
        }
    }
}

impl std::error::Error for DbError {}
impl From<std::io::Error> for DbError {
    fn from(value: std::io::Error) -> Self { Self::Io(value) }
}
pub type Result<T> = std::result::Result<T, DbError>;
