import pytest
import lize


def test_sum_as_string():
    assert lize.sum_as_string(1, 1) == "2"
