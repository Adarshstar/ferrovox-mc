use std::fmt;

#[derive(Debug)]
pub enum NbtError {
    Io(std::io::Error),
    UnknownTagId(u8),
    InvalidUtf8,
    UnexpectedEof,
}

impl fmt::Display for NbtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NbtError::Io(e) => write!(f, "io error: {e}"),
            NbtError::UnknownTagId(id) => write!(f, "unknown NBT tag id: {id}"),
            NbtError::InvalidUtf8 => write!(f, "invalid modified UTF-8 string"),
            NbtError::UnexpectedEof => write!(f, "unexpected end of NBT data"),
        }
    }
}

impl std::error::Error for NbtError {}

impl From<std::io::Error> for NbtError {
    fn from(e: std::io::Error) -> Self {
        NbtError::Io(e)
    }
}
