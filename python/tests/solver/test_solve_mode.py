from typing import get_args

import pytest
from lle.solver import SolveMode, SolveModeLiteral


def test_rust_solve_mode_rejects_invalid_lengths():
    prefixes = ["no-sequence", "no-interdependence"]
    suffixes = ["-1", "-0", "-x"]
    combinations = [f"{p}{s}" for p in prefixes for s in suffixes] + ["bogus"]
    for bad in combinations:
        with pytest.raises(ValueError):
            SolveMode.from_str(bad)
    with pytest.raises(ValueError):
        SolveMode.no_sequence(1)


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
    assert SolveMode.no_sequence().value == "no-sequence"
    assert SolveMode.no_sequence(3).value == "no-sequence-3"
    assert SolveMode.no_interdependence(4).value == "no-interdependence-4"
    # from_str is the inverse of value.
    for s in ("standard", "no-sequence", "no-sequence-3", "no-interdependence", "no-interdependence-4"):
        assert SolveMode.from_str(s).value == s
    assert SolveMode.from_str("no-interdependence-2") == SolveMode.no_interdependence(2)


def test_no_convergence_factory_and_parser_round_trip():
    """Factories and canonical strings round-trip for default and explicit thresholds."""
    default = SolveMode.no_convergence()
    explicit = SolveMode.no_convergence(3)
    assert SolveMode.from_str(default.value) == default
    assert SolveMode.from_str(explicit.value) == explicit


@pytest.mark.parametrize("k", [-1, 0, 1])
def test_no_convergence_rejects_threshold_below_two(k: int):
    with pytest.raises(ValueError):
        SolveMode.no_convergence(k)
    with pytest.raises(ValueError):
        SolveMode.from_str(f"no-convergence-{k}")


@pytest.mark.parametrize("n", [-1, 0, 1])
def test_parameterized_factories_raise_value_error_for_signed_input(n: int):
    """Negative thresholds reach the explicit check instead of failing conversion.

    @ai-generated
    """
    for factory in (SolveMode.no_sequence, SolveMode.no_interdependence, SolveMode.no_convergence):
        with pytest.raises(ValueError):
            factory(n)


def test_solve_mode_literal_includes_no_convergence():
    """The public solve-mode literal includes the base convergence mode."""
    assert "no-convergence" in get_args(SolveModeLiteral)


def test_no_divergence_factory_and_parser_round_trip():
    """Factories and canonical strings round-trip for default and explicit thresholds.

    @ai-generated
    """
    assert SolveMode.no_divergence().value == "no-divergence"
    assert SolveMode.no_divergence(3).value == "no-divergence-3"
    assert SolveMode.from_str("no-divergence-2") == SolveMode.no_divergence(2)
    assert SolveMode.from_str("no-divergence-2").value == "no-divergence"
    assert SolveMode.from_str("no-divergence-3") == SolveMode.no_divergence(3)


@pytest.mark.parametrize("k", [-1, 0, 1])
def test_no_divergence_rejects_threshold_below_two(k: int):
    """Invalid thresholds raise `ValueError`, never `OverflowError`."""
    with pytest.raises(ValueError):
        SolveMode.no_divergence(k)
    with pytest.raises(ValueError):
        SolveMode.from_str(f"no-divergence-{k}")


@pytest.mark.parametrize("bad", ["no-divergence-x", "no-divergence2", "no-divergence-"])
def test_no_divergence_rejects_malformed_strings(bad: str):
    """Malformed suffixes are not parsed as divergence modes.

    @ai-generated
    """
    with pytest.raises(ValueError):
        SolveMode.from_str(bad)


def test_no_divergence_is_distinct_from_no_convergence():
    """The two degree profiles are independent modes.

    @ai-generated
    """
    assert SolveMode.no_divergence(2) != SolveMode.no_convergence(2)
    assert hash(SolveMode.no_divergence(2)) != hash(SolveMode.no_convergence(2))


def test_solve_mode_literal_includes_no_divergence():
    """The public solve-mode literal includes the base divergence mode.

    @ai-generated
    """
    assert "no-divergence" in get_args(SolveModeLiteral)


def test_solve_accepts_the_divergence_mode_as_a_string():
    """The direct string path reaches the divergence encoding.

    @ai-generated
    """
    from lle import World, solve

    world = World("S0 . X")
    assert solve(world, 4, mode="no-divergence-3") is not None
