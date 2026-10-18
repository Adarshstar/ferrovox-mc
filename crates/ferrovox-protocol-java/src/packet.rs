//! Raw packet framing: Java Edition packets are length-prefixed
//! (`VarInt length` + `VarInt packet id` + payload). This module reads a
//! full packet body off the wire without interpreting its contents.

use ferrovox_net::{read_varint, read_varint_async, write_varint};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct RawPacket {
    pub id: i32,
    pub data: Vec<u8>,
}

/// Read one full (length-prefixed) packet from the stream and split off
/// its packet id.
pub async fn read_packet<R: AsyncRead + Unpin>(stream: &mut R) -> std::io::Result<RawPacket> {
    let len = read_varint_async(stream)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    let mut body = vec![0u8; len as usize];
    stream.read_exact(&mut body).await?;

    let (id, consumed) = read_varint(&body)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(RawPacket {
        id,
        data: body[consumed..].to_vec(),
    })
}

/// Write a packet (id + payload) to the stream with the required length
/// prefix.
pub async fn write_packet<W: AsyncWrite + Unpin>(
    stream: &mut W,
    id: i32,
    payload: &[u8],
) -> std::io::Result<()> {
    let mut id_buf = Vec::new();
    write_varint(id, &mut id_buf);

    let mut len_buf = Vec::new();
    write_varint((id_buf.len() + payload.len()) as i32, &mut len_buf);

    stream.write_all(&len_buf).await?;
    stream.write_all(&id_buf).await?;
    stream.write_all(payload).await?;
    stream.flush().await?;
    Ok(())
}

/// Helpers for reading primitive protocol types out of a packet body.
pub struct PacketReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> PacketReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn read_varint(&mut self) -> std::io::Result<i32> {
        let (v, n) = read_varint(&self.buf[self.pos..])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        self.pos += n;
        Ok(v)
    }

    pub fn read_u8(&mut self) -> std::io::Result<u8> {
        if self.pos >= self.buf.len() {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }

    pub fn read_u16(&mut self) -> std::io::Result<u16> {
        if self.pos + 2 > self.buf.len() {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let v = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    pub fn read_i32(&mut self) -> std::io::Result<i32> {
        if self.pos + 4 > self.buf.len() {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let v = i32::from_be_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    pub fn read_i64(&mut self) -> std::io::Result<i64> {
        if self.pos + 8 > self.buf.len() {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let v = i64::from_be_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
            self.buf[self.pos + 4],
            self.buf[self.pos + 5],
            self.buf[self.pos + 6],
            self.buf[self.pos + 7],
        ]);
        self.pos += 8;
        Ok(v)
    }

    pub fn read_f64(&mut self) -> std::io::Result<f64> {
        let bits = self.read_i64()?;
        Ok(f64::from_bits(bits as u64))
    }

    pub fn read_bool(&mut self) -> std::io::Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_string(&mut self) -> std::io::Result<String> {
        let len = self.read_varint()? as usize;
        if self.pos + len > self.buf.len() {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let s = String::from_utf8_lossy(&self.buf[self.pos..self.pos + len]).into_owned();
        self.pos += len;
        Ok(s)
    }

    /// Read a raw number of bytes from the buffer, advancing the position.
    pub fn read_bytes(&mut self, n: usize) -> std::io::Result<Vec<u8>> {
        if self.pos + n > self.buf.len() {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let bytes = self.buf[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(bytes)
    }
}

/// Helpers for writing primitive protocol types into a packet payload.
#[derive(Default)]
pub struct PacketWriter {
    pub buf: Vec<u8>,
}

impl PacketWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_varint(&mut self, v: i32) -> &mut Self {
        write_varint(v, &mut self.buf);
        self
    }

    pub fn write_string(&mut self, s: &str) -> &mut Self {
        self.write_varint(s.len() as i32);
        self.buf.extend_from_slice(s.as_bytes());
        self
    }

    pub fn write_u8(&mut self, v: u8) -> &mut Self {
        self.buf.push(v);
        self
    }

    pub fn write_i8(&mut self, v: i8) -> &mut Self {
        self.buf.push(v as u8);
        self
    }

    pub fn write_u16(&mut self, v: u16) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn write_i32(&mut self, v: i32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn write_i64(&mut self, v: i64) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn write_f32(&mut self, v: f32) -> &mut Self {
        self.buf.extend_from_slice(&(v.to_bits() as i32).to_be_bytes());
        self
    }

    pub fn write_f64(&mut self, v: f64) -> &mut Self {
        self.buf.extend_from_slice(&(v.to_bits() as i64).to_be_bytes());
        self
    }

    pub fn write_bool(&mut self, v: bool) -> &mut Self {
        self.buf.push(if v { 1 } else { 0 });
        self
    }

    /// Write a 16-byte UUID (128-bit).
    pub fn write_uuid_bytes(&mut self, bytes: &[u8; 16]) -> &mut Self {
        self.buf.extend_from_slice(bytes);
        self
    }
}
