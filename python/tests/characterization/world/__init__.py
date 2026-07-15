from collections.abc import Callable
from typing import ParamSpec, TypeVar

import pytest

P = ParamSpec("P")
T = TypeVar("T")


def call_or_xfail_unimplemented(function: Callable[P, T], *args: P.args, **kwargs: P.kwargs) -> T:
    """Call an endpoint and xfail only its known implementation placeholder.

    @ai-generated
    """
    try:
        return function(*args, **kwargs)
    except BaseException as error:
        is_rust_placeholder = (
            type(error).__module__ == "pyo3_runtime" and type(error).__name__ == "PanicException" and str(error) == "not yet implemented"
        )
        if isinstance(error, NotImplementedError) or is_rust_placeholder:
            pytest.xfail("no-interdependence solving mode is not implemented in Rust yet")
        raise
