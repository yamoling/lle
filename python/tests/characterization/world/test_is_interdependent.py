"""Regression/reproduction tests for the interdependence cycle-rotation bug.

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

import pytest
from lle import World, solver
from lle.characterization.plan import profile_plan
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
    assert profile_plan(world, plan).interdependence_order() >= 2
    # Earliest help edge starts at agent 0 -> canonical rotation.
    earliest = min(profile_plan(world, plan).graph.edges, key=lambda e: e.t)
    assert earliest.helper == 0

    # The forbid works for this rotation.
    assert solver.solve(world, T_MAX, mode="no-interdependence-2") is None

    wc = WorldCharacterizer(world, t_max=T_MAX)
    assert wc.is_mutual()
    assert wc.is_interdependent(2)


def test_interdependence_2_detected_in_reversed_rotation():
    """Counterexample: the reversed rotation (1 -> 0 then 0 -> 1) must also be caught.

    This world requires the exact same mutual cooperation as the positive case,
    but its only feasible order has agent 1 help agent 0 first. A correct
    ``no-interdependence-2`` must reject it just like the positive case, because a
    two-agent mutual world *is* an order-2 temporal cycle.
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
    assert profile_plan(world, plan).interdependence_order() >= 2
    # Earliest help edge starts at agent 1 -> the rotation the bug misses.
    earliest = min(profile_plan(world, plan).graph.edges, key=lambda e: e.t)
    assert earliest.helper == 1

    # The world requires a mutual / order-2 cycle, so no-interdependence-2 must
    # make it unsolvable. On the current (buggy) solver this returns a plan.
    assert solver.solve(world, T_MAX, mode="no-interdependence-2") is None

    wc = WorldCharacterizer(world, t_max=T_MAX)
    assert wc.is_mutual()
    # For a two-agent world is_interdependent(2) must equal is_mutual.
    assert wc.is_interdependent(2)


def test_is_interdependent_rejects_order_below_2():
    wc = WorldCharacterizer(World.level(1), t_max=10)
    for n_agents in [-1, 0, 1]:
        with pytest.raises(ValueError):
            wc.is_interdependent(n_agents)


def test_compute_shortest_non_interdependent_path_rejects_order_below_2():
    wc = WorldCharacterizer(World.level(1), t_max=10)
    for order in [-1, 0, 1]:
        with pytest.raises(ValueError):
            wc.compute_shortest_non_interdependent_path(order)


def test_unsolvable_world_raises_on_is_interdependent():
    world = World("S0 @ X")
    with pytest.raises(ValueError):
        _ = WorldCharacterizer(world, t_max=10).is_interdependent(2)


def test_no_3interdependent_because_of_temporality():
    """
    We want to show that temporality is important and that a temporally-flattened graph
    cannot represent the actual cooperation graph.

    The below World is organized is such that:
       - the two bottom exits are only available to agents 0 and 2 because they stand in a laser beam;
       - agents 1 and 3 have no choice but to walk on the exit tile two steps below them
       - agents 1 and 3 block a laser from their respective exit tiles.

    In this world, we have a first step where two independent help events occur:
        - help(0, 1, t=1)
        - help(2, 3, t=1)
    Then, we have agent 2 going from right to left:
        - help(3, 2, t=4)
        - help(1, 2, t=6)
        - help(1, 2, t=7,8,9,10,11)
    And finally agent 0 going from left to right
        - help(1, 0, t=8)
        - help(3, 0, t=10)
        - help(3, 0, t=11)
    In a flattened graph, we would have a cycle 0 -> 1 -> 2 -> 3 -> 0 while there is actually
    no dependency between 1 and 3.
    """
    world = World("""
 @  @  L1S @ L3S  @   @
 @  S0 S1  @ S3  S2   @
L0E .   .  @  .   .  L2W
 @  .   X  @  X   .   @
 @  .   .  .  .   .   @
 @ L2E  X  @  X  L0W  @
""")
    wc = WorldCharacterizer(world, 20)
    assert wc.is_cooperative()
    assert not wc.is_independent()
    assert not wc.is_asymmetric()
    assert wc.is_mutual()  # 0 helps 1 and vice-versa
    assert wc.is_chained(2)  # Equivalent to is_mutual
    assert not wc.is_chained(3)
    assert wc.is_interdependent(2)  # Equivalent to is_mutual
    assert not wc.is_interdependent(3)


def test_three_agent_cycle_is_3_interdependent_but_not_4_interdependent():
    """A 3-agent cycle exercises the parametrized chain/interdependence upper edge."""
    wc = WorldCharacterizer(
        World("""
     @ L0S L2S L1S .
    S0  .   .   .  X
    S1  .   .   .  X
    S2  .   .   .  X
    """),
        t_max=15,
    )
    assert wc.is_solvable()
    assert wc.is_chained(2)
    assert wc.is_chained(3)
    assert not wc.is_chained(4)
    assert wc.is_interdependent(2)
    assert wc.is_interdependent(3)
    assert not wc.is_interdependent(4)


def test_two_agent_cycle_in_three_agent_world_is_not_3_interdependent():
    """
    The world requires a mutual cycle between 0 and 2, but agent 1 can avoid joining the 3-cycle
    by taking the way on the left. Agent 1 is the only ont that can take the way left because there
    is a colour-1 laser beaming left.

    The resulting dependencies are:
        - 0 depends on 2
        - 1 depends on 2
        - 2 depends on 0
    """
    wc = WorldCharacterizer(
        World("""
     .  S1  S0 S2 @ @
     . L0E  .  .  @ @
     .  .  L1W .  . .
     @  .   .  @ . .
    L2E .   .  .  . .
     @  @   .  X  X X
    """),
        t_max=16,
    )
    assert wc.is_solvable()
    assert wc.is_chained(2)
    assert not wc.is_chained(3)
    assert wc.is_interdependent(2)
    assert not wc.is_interdependent(3)


def test_paper_example_interdependent3():
    # TODO: not tested yet
    world = World("""
        @   S0   @ L2S   S1 @
       L0E  .    .   .   .  @
        @   .    @   X   . @
        @   .    @   .   X L1W
        @   .    @   .   .  @
        @   .    .   X   S2 @""")
    wc = WorldCharacterizer(world, 10)
    assert wc.is_solvable()
    assert wc.is_chained(2)
    assert wc.is_chained(3)
    assert not wc.is_distributed(2)
    assert not wc.is_interdependent(2)
    assert wc.is_interdependent(3)


def test_paper_counter_exemple_interdependent3():
    """
    This world is not interdependent 3 because even though it looks like it is the case
    because agent 2 helps agent 0 before being helped by agent 1.

    As a result, the best characterisation is Chain(2).
    """
    # TODO: not tested yet
    world = World("""
        @  S0 @  @  L1S S1
       L0E .  .  .   .  .
        @  . S2 L2W  .  X
        @  .  .  .   .  X
       L0E .  @  .   .  .
        @  X  @  .   .  .
        """)
    wc = WorldCharacterizer(world, 10)
    assert wc.is_solvable()
    assert wc.is_chained(2)
    assert not wc.is_distributed(2)
    assert not wc.is_chained(3)
    assert not wc.is_interdependent(2)
    assert not wc.is_interdependent(3)
