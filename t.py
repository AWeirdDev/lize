from typing import Callable
from lize import serialize, deserialize, Runnable


def add(a: int, b: int, k: float) -> float:
    return (a + b) * k

print(Runnable.from_pyfn(add))
s = serialize(add)
d: Callable[[int, int, float], int] = deserialize(s)
print(d)
