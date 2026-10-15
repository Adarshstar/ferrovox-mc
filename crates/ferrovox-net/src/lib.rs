//! Low-level networking primitives shared by both protocol implementations:
//! VarInt/VarLong codecs and buffered packet framing used by the Java
//! Edition protocol (Bedrock uses RakNet framing instead, see
//! `ferrovox-protocol-bedrock`).

pub mod varint;

pub use varint::{read_varint, read_varint_async, write_varint, VarIntError};
