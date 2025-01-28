# `lize`
A very stupid way of serializing and deserializing data into bytes. I mean, the performance is essentially the same as `bincode` (which is really disappointing), so there's nothing to really see here.

`lize` supports recursive data structures.

First, we create a `Value`.

```rust
use lize::{N, Result, SmallVec, Value};

// We support HashMap's!
let value = Value::HashMap(vec![
    // Some simple values
    (Value::Slice(b"hello"), Value::Slice(b"world")),
    (Value::Slice(b"money"), Value::Int(6969694200)),
    
    // A bit more *advanced* values, :smirk:
    (
        Value::Slice(b"do_chores"),
        Value::Optional(Some(Box::new(Value::Bool(true))))
    ),

    // ...doing taxes is optional he said.
    (
        Value::Slice(b"do_taxes"),
        Value::Optional(None)
    )
]);
```

Then, we can serialize it into a buffer, then deserialize it back.

```rust
// We can serialize into a (small)vec...
let mut buf = SmallVec::<[u8; N]>::new();
value.serialize_into(&mut buf)?;

// ...and deserialize back
let deserialized = Value::deserialize_from(&buf)?;
```

Finally... `match`-ing time!

```rust
match deserialized {
    Value::Int(i) => println!("Int: {:?}", i),
    Value::Slice(s) => println!("Slice: {:?}", s),

    // ...so on and so forth

    // A Value::SliceLike will never be reached!!
    // It's designed to act like a Value::Slice, but for Vec's.
    // Therefore, it can only be matched as a Value::Slice above.
    Value::SliceLike(_) => unreachable!(),
}
```

***

(c) 2024 [AWeirdDev](https://github.com/AWeirdDev)
