# `lize`
A stupid way of serializing data into a slice. Designed for really small data.

```rust
use lize::Value;

// You can create hashmaps like so:
let value = Value::HashMap(vec![
    (Value::Slice(b"hello"), Value::Slice(b"world")),
    (Value::Slice(b"money"), Value::I64(6969694200)),
]);

// ..then serialize it into a SmallVec (recommended)
let mut buffer = lize::SmallVec::<[u8; lize::STACK_N]>::new();
value.serialize_into(&mut buffer)?;

// Alternatively, you can serialize it into a Vec.
// That'd be more convenient.
// let buffer = value.serialize()?;

// Then, we can take a look at our deserialized data.
let deserialized = Value::deserialize_from(&buffer)?;
println!("{deserialized:?}");

let mut buffer = SmallVec::<[u8; lize::STACK_N]>::new();
value.serialize_into(&mut buffer)?;
```

# `Value`
There are some other cool usages other than just hashmaps.

```rust
let value = Value::Vector(vec![
    Value::Bool(true),
    Value::Slice(b"hello"),

    Value::F64(std::f64::consts::PI),
    Value::I64(1234567890123456789),
    Value::I32(123456789),

    Value::Optional(Some(Box::new(Value::Slice(b"world")))),
    Value::Optional(None),

    Value::U8(1_u8),
    Value::SmallU8(123_u8) // Must be <= 235. Occupies a single byte.
]);
```

Of course, if you would, you can use `Value::from(...)` instead of doing that manually. Saves time!

```rust
let a = 123_i64;
let v = Value::from(a);

assert!(matches!(v, Value::I64(_)));
```

# (de)serializing directly
You can use `serialize(...)` and `deserialize(...)` to have the Bincode-like interface.

```rust
let a = 123_i64;
let ser = serialize(a)?;

let b: i64 = deserialize(&ser)?;

assert_eq!(a, b);
```

***

(c) 2025 [AWeirdDev](https://github.com/AWeirdDev)
