"""Tests for asymmetric cooperation characterization.

Asymmetric cooperation is a help edge `a -> b` where `a` is never helped by any
other agent in the trajectory. The solver mode `no-asymmetric` forbids such
edges, so a world requires asymmetric cooperation when it is solvable but has no
solution under that mode.
"""

from __future__ import annotations

import lle
from lle import World
from lle.characterization import WorldCharacterizer
from lle.characterization.trajectory import DependencyEdge, TemporalDependencyGraph
from lle.solver import Solver

from .mocks import MockSolver


def test_trajectory_profile_detects_asymmetric_edges():
    graph = TemporalDependencyGraph(
        n_agents=3,
        edges=[
            DependencyEdge(0, 1, 2),
            DependencyEdge(1, 2, 3),
        ],
        horizon=3,
    )
    assert graph.asymmetric_edges() == {(0, 1)}
    assert graph.profile().is_asymmetric


def test_trajectory_profile_rejects_mutual_as_asymmetric():
    graph = TemporalDependencyGraph(
        n_agents=2,
        edges=[DependencyEdge(0, 1, 1), DependencyEdge(1, 0, 2)],
        horizon=2,
    )
    assert graph.asymmetric_edges() == set()
    assert graph.profile().is_asymmetric is False
    assert graph.profile().is_mutual is True


def test_1_laser_world_requires_asymmetric_cooperation():
    world = World("""
     @  S0 S1
    L0E .  .
     @  X  X""")
    solver = Solver(world, 6)
    assert solver.solve() is not None
    assert solver.solve("no-asymmetric") is None
    assert lle.is_asymmetric(world, 6)
    assert lle.characterize(world, 6).is_asymmetric


def test_double_world_requires_asymmetric_cooperation():
    world = World("""
     @  S0 S1 @  @  S2 S3
    L0E .  .  @ L2E .  .
     @  X  X  @  @  X  X
    """)
    solver = Solver(world, 6)
    assert solver.solve() is not None
    assert solver.solve("no-asymmetric") is None
    assert lle.is_asymmetric(world, 6) is True


def test_independent_world_is_not_asymmetric():
    world = World("""
    S0 . S1
     . . .
     X . X""")
    solver = Solver(world, 6)
    assert solver.solve("no-asymmetric") is not None
    assert lle.is_asymmetric(world, 6) is False


def test_no_laser_world_short_circuits_without_no_asymmetric_solve():
    world = World("""
    S0 . S1
     . . .
     X . X""")
    mock_solver = MockSolver(world, 6, responses={"standard": []})
    characterizer = WorldCharacterizer(world, 6)
    characterizer._solver = mock_solver

    assert characterizer.is_asymmetric is False
    assert characterizer.shortest_non_asymmetric_path == []
    assert mock_solver.calls == ["standard"]


def test_known_independent_path_short_circuits_no_asymmetric_solve():
    world = World("""
     @  S0 S1
    L0E .  .
     @  X  X""")
    independent_path = [(lle.Action.STAY, lle.Action.STAY)]
    mock_solver = MockSolver(world, 6, responses={"no-cooperation": independent_path})
    characterizer = WorldCharacterizer(world, 6)
    characterizer._solver = mock_solver

    assert characterizer.shortest_independent_path is independent_path
    assert characterizer.shortest_non_asymmetric_path is independent_path
    assert mock_solver.calls == ["no-cooperation"]


def test_pure_mutual_world_has_non_asymmetric_solution():
    world = World("""
     S0 . . S1
    L0E . . .
     .  . . L1W
     X  . . X""")
    solver = Solver(world, 10)
    assert solver.solve() is not None
    assert solver.solve("no-asymmetric") is not None
    assert not lle.is_asymmetric(world, 10)
