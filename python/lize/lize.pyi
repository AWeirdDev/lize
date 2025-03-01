from typing import Any, Callable, Generic, NoReturn, TypeVar, Union

Value = Union[
    str,
    int,
    float,
    bool,
    list["Value"],
    dict["Value", "Value"],
    None,
    "Runnable[Any]",
    Callable[..., Any],
]

def serialize(x: Value) -> bytes: ...
def deserialize(x: bytes) -> Any: ...

T = TypeVar("T")

class Runnable(Generic[T]):
    """This does **NOT** construct this class.
    
    Use other methods instead.
    """
    def __init__(self) -> NoReturn: ...
    @staticmethod
    def from_pyfn(fn: Callable[..., T]) -> "Runnable[T]": ...
    @staticmethod
    def from_bytes(bytes: bytes) -> "Runnable[T]": ...
    def run(self, *args: Any, **kwargs: Any) -> T: ...
    def as_bytes(self) -> bytes: ...
