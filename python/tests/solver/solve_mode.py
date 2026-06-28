from typing import get_args

import pytest
from lle.solver import SolveMode, SolveModeLiteral


def test_rust_solve_mode_rejects_invalid_lengths():
    for bad in ("no-chain-1", "no-chain-0", "no-chain-x", "no-interdependence-1", "bogus"):
        with pytest.raises(ValueError):
            SolveMode.from_str(bad)
    with pytest.raises(ValueError):
        SolveMode.no_chain(1)


def test_rust_solve_mode_values():
    assert SolveMode.standard().value == "standard"
    assert SolveMode.no_cooperation().value == "no-cooperation"
    assert SolveMode.no_mutual().value == "no-interdependence"
    assert str(SolveMode.no_cooperation()) == "no-cooperation"


def test_typing_solve_mode_literal():
    for lit in get_args(SolveModeLiteral):
        SolveMode.from_str(lit)


def test_rust_solve_mode_parametrized_values_round_trip():
    # Default length renders without a suffix; explicit lengths are kept.
    assert SolveMode.no_chain().value == "no-chain"
    assert SolveMode.no_chain(3).value == "no-chain-3"
    assert SolveMode.no_interdependence(4).value == "no-interdependence-4"
    # from_str is the inverse of value.
    for s in ("standard", "no-chain", "no-chain-3", "no-interdependence", "no-interdependence-4"):
        assert SolveMode.from_str(s).value == s
    assert SolveMode.from_str("no-interdependence-2") == SolveMode.no_interdependence(2)
