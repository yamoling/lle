"""Tests for asymmetric cooperation characterization.

Asymmetric cooperation is a help edge `a -> b` where `a` is never helped by any
other agent in the trajectory. The solver mode `no-asymmetric` forbids such
edges, so a world requires asymmetric cooperation when it is solvable but has no
solution under that mode.
"""

from __future__ import annotations

import lle
import lle.characterization.world_characterization as world_characterization
from lle import World
from lle.characterization import WorldCharacterizer
from lle.characterization.trajectory import DependencyEdge, TemporalDependencyGraph
from lle.solver import solve
from pytest import MonkeyPatch


def test_trajectory_profile_detects_asymmetric_edges():
    graph = TemporalDependencyGraph(
        n_agents=3,
        edges=[DependencyEdge(0, 1, 2), DependencyEdge(1, 2, 3)],
        horizon=3,
    )
    assert graph.asymmetric_edges() == {(0, 1)}
    assert graph.profile().is_asymmetric is True


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
    assert solve(world, 6) is not None
    assert solve(world, 6, mode="no-asymmetric") is None
    assert lle.is_asymmetric(world, 6)
    assert lle.characterize(world, 6).is_asymmetric


def test_double_world_requires_asymmetric_cooperation():
    world = World("""
     @  S0 S1 @  @  S2 S3
    L0E .  .  @ L2E .  .
     @  X  X  @  @  X  X
    """)
    assert solve(world, 6) is not None
    assert solve(world, 6, mode="no-asymmetric") is None
    assert lle.is_asymmetric(world, 6) is True


def test_independent_world_is_not_asymmetric():
    world = World("""
    S0 . S1
     . . .
     X . X""")
    assert solve(world, 6, mode="no-asymmetric") is not None
    assert lle.is_asymmetric(world, 6) is False


def test_no_laser_world_short_circuits_without_no_asymmetric_solve(monkeypatch: MonkeyPatch):
    world = World("""
    S0 . S1
     . . .
     X . X""")
    calls = []

    def fake_solve(_world, _t_max, *, mode="standard"):
        calls.append(mode)
        return []

    monkeypatch.setattr(world_characterization.solver, "solve", fake_solve)
    characterizer = WorldCharacterizer(world, 6)

    assert characterizer.is_asymmetric is False
    assert characterizer.shortest_non_asymmetric_path == []
    assert calls == ["standard"]


def test_known_independent_path_short_circuits_no_asymmetric_solve(monkeypatch: MonkeyPatch):
    world = World("""
     @  S0 S1
    L0E .  .
     @  X  X""")
    calls = []
    independent_path = [(lle.Action.STAY, lle.Action.STAY)]

    def fake_solve(_world, _t_max, *, mode="standard"):
        calls.append(mode)
        if mode == "no-cooperation":
            return independent_path
        if mode == "no-asymmetric":
            raise AssertionError("no-asymmetric solve should be skipped")
        return []

    monkeypatch.setattr(world_characterization.solver, "solve", fake_solve)
    characterizer = WorldCharacterizer(world, 6)

    assert characterizer.shortest_independent_path is independent_path
    assert characterizer.shortest_non_asymmetric_path is independent_path
    assert calls == ["no-cooperation"]


def test_pure_mutual_world_has_non_asymmetric_solution():
    world = World("""
     S0 . . S1
    L0E . . .
     .  . . L1W
     X  . . X""")
    assert solve(world, 10) is not None
    assert solve(world, 10, mode="no-asymmetric") is not None
    assert not lle.is_asymmetric(world, 10)
