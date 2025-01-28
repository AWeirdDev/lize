//! A very stupid way of serializing and deserializing data into bytes.
//! 
//! ---
//! 
//! Simple (de)serialization library. Almost zero-copy... almost. The performance is essentially the same as `bincode`,
//! nothing too surprising to be honest.
//!
//! ```rust
//! use lize::{Result, Value};
//!
//! let run = || -> Result<()> {
//!     let value = Value::HashMap(vec![
//!         (Value::Slice(b"hello"), Value::Slice(b"world")),
//!         (Value::Slice(b"money"), Value::Int(6969694200)),
//!     ]);
//!
//!     // Create a new buffer
//!     // ...you could use smallvec::SmallVec, this is just a shortcut
//!     let mut buffer = lize::SmallVec::<[u8; 128]>::new();
//!
//!     // Serialize                     ↓ make sure!
//!     value.serialize_into(&mut buffer)?;
//!
//!     // Deserialize
//!     let deserialized = Value::deserialize_from(&buffer)?;
//!
//!     // Money-back gurantee :)
//!     assert_eq!(value, deserialized);
//!
//!     Ok(())
//! };
//! run();
//! ```

use std::io::Write;

pub use anyhow::Result;
pub use smallvec::SmallVec;

/// 8 * 128 = 1024
pub const N: usize = 128;

/// Represents a value.
///
/// ```rust
/// use lize::Value;
///
///
/// let number = Value::Int(6969694200);
/// let slice = Value::Slice(b"hello");
/// let vector = Value::Vector(vec![number, slice]);
///
/// // ...and more!
/// ```
#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    /// Represnts an integer.
    Int(i64),

    /// Represnts a byte slice.
    Slice(&'a [u8]),

    /// Represents a vector.
    Vector(Vec<Value<'a>>),

    /// Represents a hashmap.
    HashMap(Vec<(Value<'a>, Value<'a>)>),

    /// Represents a boolean.
    Bool(bool),

    /// Represents a float.
    Float(f64),

    /// Represents an optional value.
    Optional(Option<Box<Value<'a>>>),

    /// A slice-like. This will never be returned from `Value::deserialize_from`,
    /// since it's intended to act like a `Value::Slice` when serializing.
    /// The main difference is that you can pass an owned Vec instead of a borrowed slice.
    SliceLike(Vec<u8>),
}

impl<'a> Value<'a> {
    /// Serialize this value to a buffer.
    pub fn serialize_into(&self, cursor: &mut SmallVec<[u8; N]>) -> Result<()> {
        match self {
            Self::Int(i) => {
                cursor.write_all(&[0])?;
                cursor.write_all(&i.to_le_bytes())?;
            }
            Self::Slice(s) => {
                cursor.write_all(&[1])?;

                let ln = s.len() as u8;
                cursor.write_all(&ln.to_le_bytes())?;
                cursor.write_all(&s)?;
            }
            Self::Vector(v) => {
                cursor.write_all(&[2])?;

                for item in v {
                    let mut buf = SmallVec::<[u8; N]>::new();
                    item.serialize_into(&mut buf)?;

                    let ln = buf.len() as u8;
                    cursor.write_all(&ln.to_le_bytes())?;
                    cursor.write_all(&buf)?;
                }

                cursor.write_all(&[3])?;
            }
            Self::HashMap(h) => {
                cursor.write_all(&[4])?;

                for (key, value) in h {
                    let mut keybuf = SmallVec::<[u8; N]>::new();
                    let mut valbuf = SmallVec::<[u8; N]>::new();
                    key.serialize_into(&mut keybuf)?;
                    value.serialize_into(&mut valbuf)?;

                    let ln_key = keybuf.len() as u8;
                    cursor.write_all(&ln_key.to_le_bytes())?;
                    cursor.write_all(&keybuf)?;

                    let ln_val = valbuf.len() as u8;
                    cursor.write_all(&ln_val.to_le_bytes())?;
                    cursor.write_all(&valbuf)?;
                }

                cursor.write_all(&[5])?;
            }
            Self::Bool(b) => {
                if *b {
                    cursor.write_all(&[6])?;
                } else {
                    cursor.write_all(&[7])?;
                }
            }
            Self::Float(f) => {
                cursor.write_all(&[8])?;
                cursor.write_all(&f.to_le_bytes())?;
            }
            Self::Optional(value) => match value {
                Some(bv) => {
                    cursor.write_all(&[9])?;
                    let mut buf = SmallVec::<[u8; N]>::new();
                    bv.serialize_into(&mut buf)?;

                    let ln = buf.len() as u8;
                    cursor.write_all(&ln.to_le_bytes())?;
                    cursor.write_all(&buf)?;
                }
                None => cursor.write_all(&[10])?,
            },
            Self::SliceLike(v) => {
                cursor.write_all(&[1])?;

                let ln = v.len() as u8;
                cursor.write_all(&ln.to_le_bytes())?;
                cursor.write_all(&v)?;
            }
        }

        Ok(())
    }

    /// Deserialize from a buffer.
    pub fn deserialize_from(buffer: &'a [u8]) -> Result<Self> {
        let tag = &buffer[0];
        match tag {
            0 => {
                let i = i64::from_le_bytes(buffer[1..9].try_into()?);
                Ok(Self::Int(i))
            }
            1 => {
                let ln = u8::from_le_bytes(buffer[1..2].try_into()?) as usize;
                Ok(Self::Slice(&buffer[2..(2 + ln)]))
            }
            2 => {
                let mut offset = 1_usize;
                let mut data: Vec<Value> = vec![];

                // [
                //     0    1      2~2   |  3
                //     TAG, LEN=1, DATA  |
                //                       ^ offset = 2 + 1
                // ]
                loop {
                    let ln = u8::from_le_bytes(buffer[offset..offset + 1].try_into()?) as usize;
                    let s = &buffer[(offset + 1)..(offset + 1 + ln)];
                    data.push(Value::deserialize_from(s)?);
                    offset += 1 + ln;

                    if &buffer[offset] == &3 {
                        break;
                    }
                }

                Ok(Self::Vector(data))
            }
            4 => {
                let mut offset = 1_usize;
                let mut data: Vec<(Value, Value)> = vec![];

                loop {
                    let ln_key = u8::from_le_bytes(buffer[offset..offset + 1].try_into()?) as usize;
                    let d = &buffer[(offset + 1)..(offset + 1 + ln_key)];
                    let key = Value::deserialize_from(d)?;
                    offset += 1 + ln_key;

                    let ln_val = u8::from_le_bytes(buffer[offset..offset + 1].try_into()?) as usize;
                    let d = &buffer[(offset + 1)..(offset + 1 + ln_val)];
                    let value = Value::deserialize_from(d)?;
                    offset += 1 + ln_val;

                    data.push((key, value));

                    if &buffer[offset] == &5 {
                        break;
                    }
                }

                Ok(Value::HashMap(data))
            }
            6 => Ok(Value::Bool(true)),
            7 => Ok(Value::Bool(false)),
            8 => {
                let f = f64::from_le_bytes(buffer[1..9].try_into()?);
                Ok(Value::Float(f))
            }
            9 => {
                let ln = u8::from_le_bytes(buffer[1..2].try_into()?) as usize;
                let d = &buffer[2..(2 + ln)];
                let value = Value::deserialize_from(d)?;
                Ok(Value::Optional(Some(Box::new(value))))
            }
            10 => Ok(Value::Optional(None)),
            _ => Err(anyhow::anyhow!("Unknown tag: {}", tag)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int() -> Result<()> {
        let value = Value::Int(8787);

        let mut buffer = SmallVec::<[u8; N]>::new();
        value.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(deserialized, value);

        Ok(())
    }

    #[test]
    fn test_slice() -> Result<()> {
        let data = b"There's no time to rest!";
        let value = Value::Slice(data);

        let mut buffer = SmallVec::<[u8; N]>::new();
        value.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(deserialized, value);

        Ok(())
    }

    #[test]
    fn test_vec() -> Result<()> {
        let value = Value::Vector(vec![
            Value::Int(1234),
            Value::Slice(b"do you wanna hide a body?"),
        ]);

        let mut buffer = SmallVec::<[u8; N]>::new();
        value.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(deserialized, value);

        Ok(())
    }

    #[test]
    fn test_hashmap() -> Result<()> {
        let value = Value::HashMap(vec![
            (Value::Slice(b"hello"), Value::Slice(b"world")),
            (Value::Slice(b"money"), Value::Int(6969694200)),
        ]);

        let mut buffer = SmallVec::<[u8; N]>::new();
        value.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(value, deserialized);

        Ok(())
    }

    #[test]
    fn test_boolean() -> Result<()> {
        let data = Value::Vector(vec![Value::Bool(true), Value::Bool(false)]);

        let mut buffer = SmallVec::<[u8; N]>::new();
        data.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(deserialized, data);

        Ok(())
    }

    #[test]
    fn test_float() -> Result<()> {
        let data = Value::Float(-3.14159265358979363846);

        let mut buffer = SmallVec::<[u8; N]>::new();
        data.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(deserialized, data);

        Ok(())
    }

    #[test]
    fn test_optional() -> Result<()> {
        let data = Value::Optional(Some(Box::new(Value::Vector(vec![Value::Bool(true)]))));

        let mut buffer = SmallVec::<[u8; N]>::new();
        data.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;
        if let Value::Optional(None) = deserialized {
            return Err(anyhow::anyhow!("Expected a value"));
        }

        Ok(())
    }
}
