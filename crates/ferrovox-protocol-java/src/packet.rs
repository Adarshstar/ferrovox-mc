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

    pub fn read_u16(&mut self) -> std::io::Result<u16> {
        if self.pos + 2 > self.buf.len() {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let v = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
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

    pub fn write_u16(&mut self, v: u16) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn write_i64(&mut self, v: i64) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }
}
