"""Regression/reproduction tests for the interdependence cycle-rotation bug (F3).

`no-interdependence-n` enumerates every temporal cycle with its start fixed at
the smallest-id agent (`enumerate_directed_cycles` in
`src/solver/clauses/generator.rs`), and `walk_clauses` realizes a cycle only
when its edges fire in that linearized order with non-decreasing time. For an
order-2 cycle this means the canonical walk ``[0, 1, 0]`` can only be detected
when agent 0 helps agent 1 *before* agent 1 helps agent 0. The opposite rotation
``[1, 0, 1]`` is never enumerated, so a mutual world whose only feasible help
order is "agent 1 helps agent 0 first" slips through the forbid.

The two worlds below share the *same geometry* (a mirror of
``TWO_AGENT_MUTUAL`` in ``test_world_characterization.py``); they differ only in
which agent owns which laser, which flips the forced help order:

* ``POSITIVE`` forces order ``0 -> 1`` then ``1 -> 0`` (canonical rotation) and is
  correctly rejected by ``no-interdependence-2``.
* ``COUNTEREXAMPLE`` forces order ``1 -> 0`` then ``0 -> 1`` (the missing
  rotation); ``no-interdependence-2`` wrongly accepts it even though the plan it
  returns contains an order-2 temporal cycle.

Both worlds genuinely *require* mutual cooperation (unsolvable without
cooperation, and unsolvable under ``no-mutual``), so an order-2 temporal cycle is
unavoidable in either of them.
"""

from __future__ import annotations

from lle import World, solver
from lle.characterization.trajectory import profile_trajectory
from lle.characterization.world_characterization import WorldCharacterizer

T_MAX = 6


def _requires_mutual(world: World) -> None:
    """Sanity guard: the world genuinely requires an order-2 mutual cycle."""
    assert solver.solve(world, T_MAX) is not None, "world must be solvable"
    assert solver.solve(world, T_MAX, mode="no-cooperation") is None, "must require cooperation"
    assert solver.solve(world, T_MAX, mode="no-mutual") is None, "must require mutual cooperation"


def test_interdependence_2_detected_in_canonical_rotation():
    """Positive case: the favourable rotation (0 -> 1 then 1 -> 0) is caught.

    The shortest plan exhibits an order-2 temporal cycle whose earliest help
    edge starts at agent 0, so the canonical ``[0, 1, 0]`` walk is realizable and
    ``no-interdependence-2`` correctly makes the world unsolvable.
    """
    # Order forced: agent 0 helps agent 1 first, then agent 1 helps agent 0.
    world = World("""
     .  . . S0 S1  .  . . .
    L0E . .  .  .  @  @ @ .
     .  . @  .  . L1W . . .
     .  . .  .  .  .  . . .
     .  . .  X  X  .  . . .
    """)
    _requires_mutual(world)

    plan = solver.solve(world, T_MAX)
    assert plan is not None
    assert profile_trajectory(world, plan).interdependence_order() >= 2
    # Earliest help edge starts at agent 0 -> canonical rotation.
    earliest = min(profile_trajectory(world, plan).graph.edges, key=lambda e: e.t)
    assert earliest.helper == 0

    # The forbid works for this rotation.
    assert solver.solve(world, T_MAX, mode="no-interdependence-2") is None

    wc = WorldCharacterizer(world, t_max=T_MAX)
    assert wc.is_mutual
    assert wc.is_interdependent(2)


def test_interdependence_2_detected_in_reversed_rotation():
    """Counterexample: the reversed rotation (1 -> 0 then 0 -> 1) must also be caught.

    This world requires the exact same mutual cooperation as the positive case,
    but its only feasible order has agent 1 help agent 0 first. A correct
    ``no-interdependence-2`` must reject it just like the positive case, because a
    two-agent mutual world *is* an order-2 temporal cycle.

    This test asserts the CORRECT behaviour, so it FAILS on the current solver
    (bug F3): ``enumerate_directed_cycles`` only emits the canonical ``[0, 1, 0]``
    walk, which cannot represent the ``1 -> 0`` first rotation, so the forbid
    leaks. It will pass once the solver enumerates all cycle rotations.
    """
    world = World("""
     .  . . S1 S0  .  . . .
    L1E . .  .  .  @  @ @ .
     .  . @  .  . L0W . . .
     .  . .  .  .  .  . . .
     .  . .  X  X  .  . . .
    """)
    _requires_mutual(world)

    plan = solver.solve(world, T_MAX)
    assert plan is not None
    assert profile_trajectory(world, plan).interdependence_order() >= 2
    # Earliest help edge starts at agent 1 -> the rotation the bug misses.
    earliest = min(profile_trajectory(world, plan).graph.edges, key=lambda e: e.t)
    assert earliest.helper == 1

    # The world requires a mutual / order-2 cycle, so no-interdependence-2 must
    # make it unsolvable. On the current (buggy) solver this returns a plan.
    assert solver.solve(world, T_MAX, mode="no-interdependence-2") is None

    wc = WorldCharacterizer(world, t_max=T_MAX)
    assert wc.is_mutual
    # For a two-agent world is_interdependent(2) must equal is_mutual.
    assert wc.is_interdependent(2)
