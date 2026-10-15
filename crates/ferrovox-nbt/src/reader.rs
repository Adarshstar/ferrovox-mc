use crate::{NbtError, Tag};
use byteorder::{BigEndian, ReadBytesExt};
use std::collections::HashMap;
use std::io::Read;

/// Read a top-level named tag (e.g. the root compound of a world file):
/// returns `(name, tag)`.
pub fn read_named_tag<R: Read>(r: &mut R) -> Result<(String, Tag), NbtError> {
    let id = r.read_u8()?;
    if id == 0 {
        return Ok((String::new(), Tag::End));
    }
    let name = read_string(r)?;
    let tag = read_payload(r, id)?;
    Ok((name, tag))
}

fn read_string<R: Read>(r: &mut R) -> Result<String, NbtError> {
    let len = r.read_u16::<BigEndian>()? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|_| NbtError::InvalidUtf8)
}

fn read_payload<R: Read>(r: &mut R, id: u8) -> Result<Tag, NbtError> {
    Ok(match id {
        1 => Tag::Byte(r.read_i8()?),
        2 => Tag::Short(r.read_i16::<BigEndian>()?),
        3 => Tag::Int(r.read_i32::<BigEndian>()?),
        4 => Tag::Long(r.read_i64::<BigEndian>()?),
        5 => Tag::Float(r.read_f32::<BigEndian>()?),
        6 => Tag::Double(r.read_f64::<BigEndian>()?),
        7 => {
            let len = r.read_i32::<BigEndian>()? as usize;
            let mut v = Vec::with_capacity(len);
            for _ in 0..len {
                v.push(r.read_i8()?);
            }
            Tag::ByteArray(v)
        }
        8 => Tag::String(read_string(r)?),
        9 => {
            let elem_id = r.read_u8()?;
            let len = r.read_i32::<BigEndian>()?;
            let mut v = Vec::new();
            if elem_id != 0 {
                for _ in 0..len {
                    v.push(read_payload(r, elem_id)?);
                }
            }
            Tag::List(v)
        }
        10 => {
            let mut map = HashMap::new();
            loop {
                let child_id = r.read_u8()?;
                if child_id == 0 {
                    break;
                }
                let name = read_string(r)?;
                let value = read_payload(r, child_id)?;
                map.insert(name, value);
            }
            Tag::Compound(map)
        }
        11 => {
            let len = r.read_i32::<BigEndian>()? as usize;
            let mut v = Vec::with_capacity(len);
            for _ in 0..len {
                v.push(r.read_i32::<BigEndian>()?);
            }
            Tag::IntArray(v)
        }
        12 => {
            let len = r.read_i32::<BigEndian>()? as usize;
            let mut v = Vec::with_capacity(len);
            for _ in 0..len {
                v.push(r.read_i64::<BigEndian>()?);
            }
            Tag::LongArray(v)
        }
        other => return Err(NbtError::UnknownTagId(other)),
    })
}
