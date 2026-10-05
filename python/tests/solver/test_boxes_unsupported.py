import lle
import pytest
from lle import World
from lle.solver import Solver

BOXED = "S0 B . X"


def test_solve_raises_on_world_with_boxes():
    with pytest.raises(NotImplementedError, match="movable boxes"):
        lle.solve(World(BOXED), 10)


def test_solver_constructor_raises_on_world_with_boxes():
    with pytest.raises(NotImplementedError, match="movable boxes"):
        Solver(World(BOXED), 10)


def test_characterize_raises_on_world_with_boxes():
    with pytest.raises(NotImplementedError, match="movable boxes"):
        lle.characterize(World(BOXED), 10)


def test_is_cooperative_raises_on_world_with_boxes():
    with pytest.raises(NotImplementedError, match="movable boxes"):
        lle.is_cooperative(World(BOXED), 10)


def test_world_without_boxes_still_solves():
    plan = lle.solve(World("S0 . . X"), 15, path_length=3)
    assert plan is not None
    assert len(plan) == 3
