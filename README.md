# lize
Serialize/deserialize data into bytes. Designed for **really** small data.

- **[🦀 Rust `lize`](https://github.com/AWeirdDev/lize/tree/main/lize)**
- [🐍 Python](https://github.com/AWeirdDev/lize)
- [🟡 PyPi](https://pypi.org/project/lize/)

## Python

```python
from lize import deserialize, serialize

# You can serialize numbers, strings and more.
s = serialize(["Hello, World!", 100, 3.14, {"python": "cool"}])

# ..and then deserialize it
d: list = deserialize(s)
```

Additionally, you can also serialize and deserialize **functions**. Again, small functions.

```python
from typing import Callable

def add(a: int, b: int, k: float) -> float:
    return (a + b) * k

s = serialize(add)
d: Callable[[int, int, float], int] = deserialize(s)

print(d)
# Runnable(<marshal> add(...) -> ?)
```