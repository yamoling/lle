import pytest
from lle import World, solve
from lle.characterization import profile_plan
from lle.solver import SolveMode, Solver

from ..world_layouts import LEVEL_6, ChainedCase, chained_cases


@pytest.mark.parametrize("test_case", chained_cases(), ids=lambda case: case.id)
def test_no_chain_mode_matches_world_specification(test_case: ChainedCase):
    """The mode is unsatisfiable exactly when the chain length is unavoidable."""
    plan = solve(
        test_case.layout.world(),
        test_case.t_max,
        mode=SolveMode.no_chain(test_case.length),
    )
    assert (plan is None) is test_case.expected, f"Plan: {plan}"
    if plan is not None:
        assert not profile_plan(test_case.layout.world(), plan).is_chained(test_case.length)


def test_no_chain_cache_matches_fresh_solver_across_lengths_and_horizons():
    """Reuse chain clauses without leaking cached prefixes between parameters."""
    # Unlike the fresh solver below, this one retains its ClauseGenerator caches between calls.
    shared_solver = Solver(LEVEL_6.world(), 21)

    # | Mode          | Horizon | Why                                               |
    # |---------------|---------|---------------------------------------------------|
    # | standard      | 21      | Populate shared domain clauses.                   |
    # | no_chain(2)   | 21      | Populate the length-2 chain-clause cache.         |
    # | no_chain(3)   | 21      | Ensure length 3 uses an independent cache.        |
    # | no_chain(2)   | 20      | Check the cached prefix is correct at a horizon.  |
    # | no_chain(2)   | 21      | Check the original cache remains usable.          |
    for mode, horizon, chain_length in (
        (SolveMode.standard(), 21, None),
        (SolveMode.no_chain(2), 21, 2),
        (SolveMode.no_chain(3), 21, 3),
        (SolveMode.no_chain(2), 20, 2),
        (SolveMode.no_chain(2), 21, 2),
    ):
        cached_plan = shared_solver.solve(mode, override_t_max=horizon)
        # A fresh solver is the reference result because it has no cached clauses.
        fresh_plan = Solver(LEVEL_6.world(), 21).solve(mode, override_t_max=horizon)

        # Caching must not change whether the SAT query is solvable or its shortest plan length.
        assert (cached_plan is None) is (fresh_plan is None)
        if cached_plan is not None:
            assert fresh_plan is not None
            assert len(cached_plan) == len(fresh_plan)
            if chain_length is not None:
                # Independently replay the result to verify that it obeys the requested bound.
                assert not profile_plan(LEVEL_6.world(), cached_plan).is_chained(chain_length)


def test_no_chain_five_finds_short_plan_before_larger_unsatisfiable_horizons():
    """Ascending search finds the recorded SAT-to-UNSAT horizon counterexample.

    @ai-generated
    """
    world_text = """
L0E S0 S1 L1W
L0E X  X  L1W
"""
    mode = SolveMode.no_chain(5)
    solver = Solver(World(world_text), 6)

    plan = solver.solve(mode)

    assert plan is not None
    assert len(plan) == 1
    profile = profile_plan(World(world_text), plan)
    assert profile.is_chained(4)
    assert not profile.is_chained(5)
    assert solver.solve(mode, t_min=2) is None


def test_no_chain_five_exact_horizon_sequence_is_not_upward_monotone():
    """The recorded world is SAT only at exact horizon one for `no-chain-5`.

    @ai-generated
    """
    world_text = """
L0E S0 S1 L1W
L0E X  X  L1W
"""
    mode = SolveMode.no_chain(5)
    solver = Solver(World(world_text), 6)

    exact_sat = [solver.solve(mode, t_min=horizon, override_t_max=horizon) is not None for horizon in range(1, 7)]

    assert exact_sat == [True, False, False, False, False, False]
