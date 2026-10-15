use std::fmt;
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Debug)]
pub enum VarIntError {
    TooLong,
    Io(std::io::Error),
}

impl fmt::Display for VarIntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VarIntError::TooLong => write!(f, "VarInt is too long (>5 bytes)"),
            VarIntError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}
impl std::error::Error for VarIntError {}
impl From<std::io::Error> for VarIntError {
    fn from(e: std::io::Error) -> Self {
        VarIntError::Io(e)
    }
}

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

/// Synchronously read a VarInt from an in-memory byte slice, returning the
/// value and number of bytes consumed.
pub fn read_varint(buf: &[u8]) -> Result<(i32, usize), VarIntError> {
    let mut value: i32 = 0;
    let mut position = 0u32;
    let mut index = 0usize;

    loop {
        if index >= buf.len() {
            return Err(VarIntError::Io(std::io::Error::from(
                std::io::ErrorKind::UnexpectedEof,
            )));
        }
        let byte = buf[index];
        index += 1;
        value |= ((byte & SEGMENT_BITS) as i32) << position;

        if byte & CONTINUE_BIT == 0 {
            break;
        }
        position += 7;
        if position >= 32 {
            return Err(VarIntError::TooLong);
        }
    }
    Ok((value, index))
}

/// Asynchronously read a VarInt directly off a socket/stream (used for
/// reading the length prefix of incoming Java Edition packets).
pub async fn read_varint_async<R: AsyncRead + Unpin>(r: &mut R) -> Result<i32, VarIntError> {
    let mut value: i32 = 0;
    let mut position = 0u32;

    loop {
        let byte = r.read_u8().await?;
        value |= ((byte & SEGMENT_BITS) as i32) << position;

        if byte & CONTINUE_BIT == 0 {
            break;
        }
        position += 7;
        if position >= 32 {
            return Err(VarIntError::TooLong);
        }
    }
    Ok(value)
}

/// Encode a VarInt into the provided buffer, returning the number of bytes
/// written.
pub fn write_varint(mut value: i32, out: &mut Vec<u8>) -> usize {
    let mut written = 0;
    loop {
        let mut byte = (value as u32 & SEGMENT_BITS as u32) as u8;
        value = ((value as u32) >> 7) as i32;
        if value != 0 {
            byte |= CONTINUE_BIT;
        }
        out.push(byte);
        written += 1;
        if value == 0 {
            break;
        }
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_varint() {
        for &v in &[0, 1, -1, 127, 128, 255, 25565, i32::MAX, i32::MIN] {
            let mut buf = Vec::new();
            write_varint(v, &mut buf);
            let (decoded, consumed) = read_varint(&buf).unwrap();
            assert_eq!(decoded, v);
            assert_eq!(consumed, buf.len());
        }
    }
}
