use crate::{NbtError, Tag};
use byteorder::{BigEndian, WriteBytesExt};
use std::io::Write;

/// Write a top-level named tag.
pub fn write_named_tag<W: Write>(w: &mut W, name: &str, tag: &Tag) -> Result<(), NbtError> {
    w.write_u8(tag.id())?;
    if tag.id() == 0 {
        return Ok(());
    }
    write_string(w, name)?;
    write_payload(w, tag)
}

fn write_string<W: Write>(w: &mut W, s: &str) -> Result<(), NbtError> {
    let bytes = s.as_bytes();
    w.write_u16::<BigEndian>(bytes.len() as u16)?;
    w.write_all(bytes)?;
    Ok(())
}

fn write_payload<W: Write>(w: &mut W, tag: &Tag) -> Result<(), NbtError> {
    match tag {
        Tag::End => {}
        Tag::Byte(v) => w.write_i8(*v)?,
        Tag::Short(v) => w.write_i16::<BigEndian>(*v)?,
        Tag::Int(v) => w.write_i32::<BigEndian>(*v)?,
        Tag::Long(v) => w.write_i64::<BigEndian>(*v)?,
        Tag::Float(v) => w.write_f32::<BigEndian>(*v)?,
        Tag::Double(v) => w.write_f64::<BigEndian>(*v)?,
        Tag::ByteArray(v) => {
            w.write_i32::<BigEndian>(v.len() as i32)?;
            for b in v {
                w.write_i8(*b)?;
            }
        }
        Tag::String(s) => write_string(w, s)?,
        Tag::List(items) => {
            let elem_id = items.first().map(Tag::id).unwrap_or(0);
            w.write_u8(elem_id)?;
            w.write_i32::<BigEndian>(items.len() as i32)?;
            for item in items {
                write_payload(w, item)?;
            }
        }
        Tag::Compound(map) => {
            for (name, value) in map {
                w.write_u8(value.id())?;
                write_string(w, name)?;
                write_payload(w, value)?;
            }
            w.write_u8(0)?; // TAG_End
        }
        Tag::IntArray(v) => {
            w.write_i32::<BigEndian>(v.len() as i32)?;
            for i in v {
                w.write_i32::<BigEndian>(*i)?;
            }
        }
        Tag::LongArray(v) => {
            w.write_i32::<BigEndian>(v.len() as i32)?;
            for i in v {
                w.write_i64::<BigEndian>(*i)?;
            }
        }
    }
    Ok(())
}
