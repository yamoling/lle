import lle
import pytest
from lle import World
from lle.solver import Solver


@pytest.mark.parametrize(
    ("level", "t_max", "expect_coop"),
    [
        (1, 10, False),
        (2, 10, False),
        (3, 10, True),
        (4, 10, True),
        (5, 19, True),
        (6, 21, True),
    ],
)
def test_no_cooperation_on_std_levels(level: int, t_max: int, expect_coop: bool):
    """solve(mode='no-cooperation') must agree with is_cooperative for all canonical levels."""
    world = World.level(level)
    no_coop = lle.solve(world, t_max, mode="no-cooperation")
    assert (no_coop is None) == expect_coop


def test_reusable_solver_across_modes():
    world = World("""
     .   .  S0  S1  .   .
    L0E  .   .   .  @   .
     .   .   .   .  .   .
     .   .   .   .  .   .
     X   X   .   .  .   .
    """)
    solver = Solver(world, 10)
    standard = solver.solve("standard")
    no_cooperation = solver.solve("no-cooperation")
    assert standard is not None
    assert no_cooperation is not None
    assert len(standard) < len(no_cooperation)


def test_solve_no_cooperation():
    world = World("""
     .   .  S0  S1  .   .
    L0E  .   .   .  @   .
     .   .   .   .  .   .
     .   .   .   .  .   .
     X   X   .   .  .   .
    """)
    # Agents 1 must go around the laser via (1,5), requiring at least 10 steps
    assert lle.solve(world, 9, mode="no-cooperation") is None
    assert lle.solve(world, 10, mode="no-cooperation") is not None
