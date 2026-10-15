//! Minimal NBT (Named Binary Tag) reader/writer used by the Java Edition
//! protocol and world storage. Implements the big-endian, uncompressed
//! wire format described at <https://minecraft.wiki/w/NBT_format>.
//!
//! This is a from-scratch implementation (no external NBT crate) so we
//! fully control encoding edge cases as the format evolves.

mod error;
mod reader;
mod tag;
mod writer;

pub use error::NbtError;
pub use reader::read_named_tag;
pub use tag::Tag;
pub use writer::write_named_tag;
