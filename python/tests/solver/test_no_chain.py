import pytest
from lle import solve
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
        cached_plan = shared_solver.solve(path_length=horizon, mode=mode)
        # A fresh solver is the reference result because it has no cached clauses.
        fresh_plan = Solver(LEVEL_6.world(), 21).solve(path_length=horizon, mode=mode)

        # Caching must not change whether the exact-length SAT query is solvable.
        assert (cached_plan is None) is (fresh_plan is None)
        if cached_plan is not None:
            assert fresh_plan is not None
            assert len(cached_plan) == len(fresh_plan)
            if chain_length is not None:
                # Independently replay the result to verify that it obeys the requested bound.
                assert not profile_plan(LEVEL_6.world(), cached_plan).is_chained(chain_length)
