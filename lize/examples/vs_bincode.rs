use std::time::Instant;

use lize::{Result, SmallVec, Value, STACK_N};

fn main() -> Result<()> {
    let value = Value::Vector(vec![Value::Slice(b"hello"), Value::Slice(b"world")]);
    let instant = Instant::now();
    let mut buf = SmallVec::<[u8; STACK_N]>::new();
    value.serialize_into(&mut buf)?;

    let elapsed = instant.elapsed();
    println!("Elapsed (lize): {:.2?}", elapsed);
    println!("{buf:?}");

    let instant = Instant::now();

    let value = vec!["hello", "world"];
    let buf2 = bincode::serialize(&value)?;
    let elapsed = instant.elapsed();
    println!("Elapsed (bincode): {:.2?}", elapsed);

    println!("{buf2:?}");

    Ok(())
}
