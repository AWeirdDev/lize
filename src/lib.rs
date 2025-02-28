//! A very stupid way of serializing and deserializing data into bytes.

use std::io::Write;

pub use anyhow::Result;
pub use smallvec::SmallVec;

pub const STACK_N: usize = 128;

/// Represents a value.
///
/// # Example
/// ```rust
/// use lize::Value;
///
/// let value = Value::from(1234_i64);
/// assert_eq!(value, Value::I64(1234));
/// ```
#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    /// A 64-bit signed integer. (code: `0`)
    I64(i64),

    /// A slice of bytes. (code: `1`)
    Slice(&'a [u8]),

    /// A vector of values. (code: `2`, ends with `3`)
    Vector(Vec<Value<'a>>),

    /// A map of values. (code: `4`, ends with `5`)
    HashMap(Vec<(Value<'a>, Value<'a>)>),

    /// A boolean. (code: `6`, `7`)
    Bool(bool),

    /// A 64-bit float. (code: `8`)
    F64(f64),

    /// An optional value. (code: `9`, `10` for `None`)
    Optional(Option<Box<Value<'a>>>),

    /// A `SliceLike` acts like a slice when deserializing, which will be never reached.
    /// This is created for uses where you cannot use the reference of the slice or lifetime errors.
    SliceLike(Vec<u8>),

    /// A 32-bit signed integer. (code: `11`)
    I32(i32),

    /// A 32-bit float. (code: `12`)
    F32(f32),

    /// A 8-bit unsigned integer. (code: `13`)
    U8(u8),

    /// A small u8. Must be <= 235. Occupies a single byte.
    SmallU8(u8),
}

impl<'a> Value<'a> {
    /// Creates a new value.
    pub fn new<T>(x: T) -> Self
    where
        T: Into<Value<'a>>,
    {
        x.into()
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut buf = SmallVec::<[u8; STACK_N]>::new();
        self.serialize_into(&mut buf)?;

        Ok(buf.to_vec())
    }

    pub fn serialize_into(&self, buffer: &mut SmallVec<[u8; STACK_N]>) -> Result<()> {
        match self {
            Self::I64(i) => {
                buffer.write_all(&[0])?;
                buffer.write_all(&i.to_le_bytes())?;
            }
            Self::Slice(s) => {
                buffer.write_all(&[1])?;

                let ln = s.len() as u8;
                buffer.write_all(&ln.to_le_bytes())?;
                buffer.write_all(&s)?;
            }
            Self::Vector(v) => {
                buffer.write_all(&[2])?;

                for item in v {
                    let mut buf = SmallVec::<[u8; STACK_N]>::new();
                    item.serialize_into(&mut buf)?;

                    let ln = buf.len() as u8;
                    buffer.write_all(&ln.to_le_bytes())?;
                    buffer.write_all(&buf)?;
                }

                buffer.write_all(&[3])?;
            }
            Self::HashMap(h) => {
                buffer.write_all(&[4])?;

                for (key, value) in h {
                    let mut keybuf = SmallVec::<[u8; STACK_N]>::new();
                    let mut valbuf = SmallVec::<[u8; STACK_N]>::new();
                    key.serialize_into(&mut keybuf)?;
                    value.serialize_into(&mut valbuf)?;

                    let ln_key = keybuf.len() as u8;
                    buffer.write_all(&ln_key.to_le_bytes())?;
                    buffer.write_all(&keybuf)?;

                    let ln_val = valbuf.len() as u8;
                    buffer.write_all(&ln_val.to_le_bytes())?;
                    buffer.write_all(&valbuf)?;
                }

                buffer.write_all(&[5])?;
            }
            Self::Bool(b) => {
                if *b {
                    buffer.write_all(&[6])?;
                } else {
                    buffer.write_all(&[7])?;
                }
            }
            Self::F64(f) => {
                buffer.write_all(&[8])?;
                buffer.write_all(&f.to_le_bytes())?;
            }
            Self::Optional(value) => match value {
                Some(bv) => {
                    buffer.write_all(&[9])?;
                    let mut buf = SmallVec::<[u8; STACK_N]>::new();
                    bv.serialize_into(&mut buf)?;

                    let ln = buf.len() as u8;
                    buffer.write_all(&ln.to_le_bytes())?;
                    buffer.write_all(&buf)?;
                }
                None => buffer.write_all(&[10])?,
            },
            Self::SliceLike(v) => {
                buffer.write_all(&[1])?;

                let ln = v.len() as u8;
                buffer.write_all(&ln.to_le_bytes())?;
                buffer.write_all(&v)?;
            }
            Self::I32(i) => {
                buffer.write_all(&[11])?;
                buffer.write_all(&i.to_le_bytes())?;
            }
            Self::F32(f) => {
                buffer.write_all(&[12])?;
                buffer.write_all(&f.to_le_bytes())?;
            }
            Self::U8(u) => {
                buffer.write_all(&[13])?;
                buffer.write_all(&u.to_le_bytes())?;
            }
            Self::SmallU8(u) => {
                // 20 because we may never reach there.
                if u > &235 {
                    return Err(anyhow::anyhow!("SmallU8 must be less than or equal to 235"));
                }
                buffer.write_all(&(u + 20).to_le_bytes())?;
            }
        }

        Ok(())
    }

    pub fn deserialize_from(slice: &'a [u8]) -> Result<Self> {
        let tag = &slice[0];
        match tag {
            0 => {
                let i = i64::from_le_bytes(slice[1..9].try_into()?);
                Ok(Self::I64(i))
            }
            1 => {
                let ln = u8::from_le_bytes(slice[1..2].try_into()?) as usize;
                Ok(Self::Slice(&slice[2..(2 + ln)]))
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
                    let ln = u8::from_le_bytes(slice[offset..offset + 1].try_into()?) as usize;
                    let s = &slice[(offset + 1)..(offset + 1 + ln)];
                    data.push(Value::deserialize_from(s)?);
                    offset += 1 + ln;

                    if &slice[offset] == &3 {
                        break;
                    }
                }

                Ok(Self::Vector(data))
            }
            4 => {
                let mut offset = 1_usize;
                let mut data: Vec<(Value, Value)> = vec![];

                loop {
                    let ln_key = u8::from_le_bytes(slice[offset..offset + 1].try_into()?) as usize;
                    let d = &slice[(offset + 1)..(offset + 1 + ln_key)];
                    let key = Value::deserialize_from(d)?;
                    offset += 1 + ln_key;

                    let ln_val = u8::from_le_bytes(slice[offset..offset + 1].try_into()?) as usize;
                    let d = &slice[(offset + 1)..(offset + 1 + ln_val)];
                    let value = Value::deserialize_from(d)?;
                    offset += 1 + ln_val;

                    data.push((key, value));

                    if &slice[offset] == &5 {
                        break;
                    }
                }

                Ok(Value::HashMap(data))
            }
            6 => Ok(Value::Bool(true)),
            7 => Ok(Value::Bool(false)),
            8 => {
                let f = f64::from_le_bytes(slice[1..9].try_into()?);
                Ok(Value::F64(f))
            }
            9 => {
                let ln = u8::from_le_bytes(slice[1..2].try_into()?) as usize;
                let d = &slice[2..(2 + ln)];
                let value = Value::deserialize_from(d)?;
                Ok(Value::Optional(Some(Box::new(value))))
            }
            10 => Ok(Value::Optional(None)),
            11 => {
                let i = i32::from_le_bytes(slice[1..5].try_into()?);
                Ok(Value::I32(i))
            }
            12 => {
                let f = f32::from_le_bytes(slice[1..5].try_into()?);
                Ok(Value::F32(f))
            }
            13 => Ok(Value::U8(u8::from_le_bytes(slice[1..2].try_into()?))),
            _ if tag >= &20 => Ok(Value::SmallU8(tag - 20)),
            _ => Err(anyhow::anyhow!("Unknown tag: {}", tag)),
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I64(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Value::I32(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::F64(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::F32(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_slice(&self) -> Option<&'a [u8]> {
        match self {
            Value::Slice(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_vec_for_slice(&self) -> Option<Vec<u8>> {
        match self {
            Value::Slice(s) => Some(s.to_vec()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            Value::Slice(s) => Some(std::str::from_utf8(s).ok()?),
            _ => None,
        }
    }

    pub fn as_u8(&self) -> Option<u8> {
        match self {
            Value::U8(u) => Some(*u),
            Value::SmallU8(u) => Some(*u),
            _ => None,
        }
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Self {
        Value::Slice(s.as_bytes())
    }
}

impl<'a> From<Value<'a>> for &'a str {
    fn from(value: Value<'a>) -> Self {
        match value {
            Value::Slice(s) => std::str::from_utf8(s).unwrap(),
            _ => unreachable!(),
        }
    }
}

impl<'a> From<&'a [u8]> for Value<'a> {
    fn from(s: &'a [u8]) -> Self {
        Value::Slice(s)
    }
}

impl<'a> From<Value<'a>> for &'a [u8] {
    fn from(value: Value<'a>) -> Self {
        match value {
            Value::Slice(s) => s,
            _ => unreachable!(),
        }
    }
}

impl From<i64> for Value<'_> {
    fn from(i: i64) -> Self {
        Value::I64(i)
    }
}

impl From<Value<'_>> for i64 {
    fn from(value: Value<'_>) -> Self {
        value.as_i64().unwrap()
    }
}

impl From<u8> for Value<'_> {
    fn from(i: u8) -> Self {
        Value::U8(i)
    }
}

impl From<Value<'_>> for u8 {
    fn from(value: Value<'_>) -> Self {
        value.as_u8().unwrap()
    }
}

impl From<i32> for Value<'_> {
    fn from(i: i32) -> Self {
        Value::I32(i)
    }
}

impl From<f32> for Value<'_> {
    fn from(f: f32) -> Self {
        Value::F32(f)
    }
}

impl From<Value<'_>> for f32 {
    fn from(value: Value<'_>) -> Self {
        value.as_f32().unwrap()
    }
}

impl From<f64> for Value<'_> {
    fn from(f: f64) -> Self {
        Value::F64(f)
    }
}

impl From<Value<'_>> for f64 {
    fn from(value: Value<'_>) -> Self {
        value.as_f64().unwrap()
    }
}

impl From<bool> for Value<'_> {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl<'a, T> From<Option<T>> for Value<'a>
where
    Value<'a>: From<T>,
{
    fn from(o: Option<T>) -> Self {
        Value::Optional(o.map(|item| Box::new(Value::from(item))))
    }
}

impl<'a> From<std::collections::HashMap<Value<'a>, Value<'a>>> for Value<'a> {
    fn from(m: std::collections::HashMap<Value<'a>, Value<'a>>) -> Self {
        Value::HashMap(m.into_iter().collect())
    }
}

impl<'a, K, V> From<Value<'a>> for std::collections::HashMap<K, V>
where
    K: From<Value<'a>> + std::hash::Hash + Eq,
    V: From<Value<'a>>,
{
    fn from(value: Value<'a>) -> Self {
        match value {
            Value::HashMap(m) => {
                let mut map = std::collections::HashMap::new();

                for (k, v) in m {
                    map.insert(K::from(k), V::from(v));
                }

                map
            }
            _ => unreachable!(),
        }
    }
}

impl<'a, T> From<Vec<T>> for Value<'a>
where
    Value<'a>: From<T>,
{
    fn from(v: Vec<T>) -> Self {
        Value::Vector(v.into_iter().map(|item| Value::from(item)).collect())
    }
}

impl<'a, T> From<Value<'a>> for Vec<T>
where
    T: From<Value<'a>>,
{
    fn from(value: Value<'a>) -> Self {
        match value {
            Value::Vector(v) => v.into_iter().map(|item| T::from(item)).collect(),
            _ => unreachable!(),
        }
    }
}

/// Serialize a value.
///
/// # Example
///
/// ```ignore
/// let a: i64 = 100;
/// let b: i64 = deserialize(&serialize(a)?)?;
///
/// assert_eq!(a, b);
/// ```
pub fn serialize<'a, T>(d: T) -> Result<Vec<u8>>
where
    Value<'a>: From<T>,
{
    let v = Value::from(d);
    v.serialize()
}

/// Deserialize a value.
///
/// # Example
///
/// ```ignore
/// let a: i64 = 100;
/// let b: i64 = deserialize(&serialize(a)?)?;
///
/// assert_eq!(a, b);
/// ```
pub fn deserialize<'a, T>(bytes: &'a [u8]) -> Result<T>
where
    T: From<Value<'a>>,
{
    let v = Value::deserialize_from(bytes)?;
    Ok(v.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int() -> Result<()> {
        let value = Value::I64(8787);

        let mut buffer = SmallVec::<[u8; STACK_N]>::new();
        value.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(deserialized, value);

        Ok(())
    }

    #[test]
    fn test_slice() -> Result<()> {
        let data = b"There's no time to rest!";
        let value = Value::Slice(data);

        let mut buffer = SmallVec::<[u8; STACK_N]>::new();
        value.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(deserialized, value);

        Ok(())
    }

    #[test]
    fn test_vec() -> Result<()> {
        let value = Value::Vector(vec![
            Value::I64(1234),
            Value::Slice(b"do you wanna hide a body?"),
        ]);

        let mut buffer = SmallVec::<[u8; STACK_N]>::new();
        value.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(deserialized, value);

        Ok(())
    }

    #[test]
    fn test_hashmap() -> Result<()> {
        let value = Value::HashMap(vec![
            (Value::Slice(b"hello"), Value::Slice(b"world")),
            (Value::Slice(b"money"), Value::I64(6969694200)),
        ]);

        let mut buffer = SmallVec::<[u8; STACK_N]>::new();
        value.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(value, deserialized);

        Ok(())
    }

    #[test]
    fn test_boolean() -> Result<()> {
        let data = Value::Vector(vec![Value::Bool(true), Value::Bool(false)]);

        let mut buffer = SmallVec::<[u8; STACK_N]>::new();
        data.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(deserialized, data);

        Ok(())
    }

    #[test]
    fn test_float() -> Result<()> {
        let data = Value::F64(-3.14159265358979363846);

        let mut buffer = SmallVec::<[u8; STACK_N]>::new();
        data.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(deserialized, data);

        Ok(())
    }

    #[test]
    fn test_optional() -> Result<()> {
        let data = Value::Optional(Some(Box::new(Value::Vector(vec![Value::Bool(true)]))));

        let mut buffer = SmallVec::<[u8; STACK_N]>::new();
        data.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;
        if let Value::Optional(None) = deserialized {
            return Err(anyhow::anyhow!("Expected a value"));
        }

        Ok(())
    }

    #[test]
    fn test_small_u8() -> Result<()> {
        let data = Value::SmallU8(2);

        let mut buffer = SmallVec::<[u8; STACK_N]>::new();
        data.serialize_into(&mut buffer)?;

        let deserialized = Value::deserialize_from(&buffer)?;

        assert_eq!(deserialized, data);

        Ok(())
    }

    #[test]
    fn test_from() -> Result<()> {
        let a = 123_i64;
        let v = Value::from(a);

        assert!(matches!(v, Value::I64(_)));

        Ok(())
    }

    #[test]
    fn test_serde() -> Result<()> {
        let a = vec![123_i64];
        let b: Vec<i64> = deserialize(&serialize(a.clone())?)?;

        assert_eq!(a, b);
        Ok(())
    }
}
