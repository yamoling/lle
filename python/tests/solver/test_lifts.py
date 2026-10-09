"""SAT solving of worlds with lifts, mixed with buttons, boxes and lasers.

Each expected length is the length of the shortest plan found by an exhaustive search over `World`.
The solver must find no shorter plan, a plan of exactly that length (or none at all within
`MAX_LENGTH` when the world is unsolvable), and that plan must replay in `World` without an
unavailable action or a death.
"""

from dataclasses import dataclass

import lle
import pytest
from lle import Action, EventType, World
from lle.solver import Solver

MAX_LENGTH = 10


@dataclass(frozen=True)
class Case:
    id: str
    map: str
    shortest: int | None
    """Length of the shortest plan, `None` if there is none within `MAX_LENGTH` steps."""
    trigger: bool = False
    """Whether the shortest plan must press a button."""
    collect_gems: bool = False
    max_length: int = MAX_LENGTH
    """Horizon up to which an unsolvable world was proven unsolvable."""


def is_done(world: World, collect_gems: bool):
    return all(agent.has_arrived for agent in world.agents) and (not collect_gems or world.gems_collected == world.n_gems)


def assert_plan_is_valid(map_str: str, plan, collect_gems: bool):
    world = World(map_str)
    world.reset()
    for step, joint_action in enumerate(plan):
        available = world.available_actions()
        for agent, action in enumerate(joint_action):
            assert action in available[agent], f"step {step}: {action} unavailable for agent {agent}"
        events = world.step(list(joint_action))
        assert all(event.event_type != EventType.AGENT_DIED for event in events), f"an agent died at step {step}"
    assert is_done(world, collect_gems)


def check(case: Case):
    solver = Solver(World(case.map), case.max_length + 2)
    last_failure = case.max_length if case.shortest is None else case.shortest - 1
    for length in range(1, last_failure + 1):
        assert solver.solve(path_length=length, collect_gems=case.collect_gems) is None, f"plan of length {length}"
    if case.shortest is None:
        return
    plan = solver.solve(path_length=case.shortest, collect_gems=case.collect_gems)
    assert plan is not None
    assert_plan_is_valid(case.map, plan, case.collect_gems)
    if case.trigger:
        assert any(Action.TRIGGER in joint_action for joint_action in plan)


CASES = [
    Case(
        "ride-up",
        """
        S0 TU0 B0 S1 X
        ;
        X  .   .  .  .
        """,
        shortest=4,
        trigger=True,
    ),
    Case(
        "ride-down",
        """
        X  .   .  .  .
        ;
        S0 TD0 B0 S1 X
        """,
        shortest=4,
        trigger=True,
    ),
    Case(
        "land-on-own-beam",
        """
        S0  TU0 B0 S1 X
        .   .   .  .  .
        ;
        L0E .   .  .  .
        .   .   .  .  X
        """,
        shortest=6,
        trigger=True,
    ),
    Case(
        # Landing on a beam of another colour kills the rider, and nobody can block it.
        "land-on-foreign-beam",
        """
        S0  TU0 B0 S1 X
        ;
        L1E .   .  .  X
        """,
        shortest=None,
    ),
    Case(
        # The owner of the beam upstairs blocks it so that the rider can land safely.
        "land-behind-owner-block",
        """
        S0  .  TU0 B0 S2
        .   .  .   .  X
        ;
        L1E S1 .   .  X
        .   .  .   .  X
        """,
        shortest=6,
        trigger=True,
    ),
    Case(
        # A box upstairs blocks a foreign beam, so the landing cell is safe.
        "land-behind-box-block",
        """
        S0  .  TU0 B0 S1
        .   .  .   .  X
        ;
        L1E #  .   X  .
        .   .  .   .  .
        """,
        shortest=4,
        trigger=True,
    ),
    Case(
        "lift-restricted-to-presser",
        """
        S0 TU0A1 B0 S1 X
        ;
        X  .     .  .  .
        """,
        shortest=None,
    ),
    Case(
        "lift-restricted-swap-roles",
        """
        S1 TU0A1 B0 S0 X
        ;
        X  .     .  .  .
        """,
        shortest=4,
        trigger=True,
    ),
    Case(
        # Only agent 0 may press, so agent 1 rides: they must cross each other on the second row.
        "button-restricted",
        """
        S0 TU0 B0A0 S1 X
        .  .   .    .  .
        ;
        X  .   .    .  .
        .  .   .    .  .
        """,
        shortest=6,
        trigger=True,
    ),
    Case(
        "one-press-two-lifts",
        """
        S0 TU0 B0 TU0 S1
        .  .   S2 .   X
        ;
        X  .   .  .   X
        .  .   .  .   .
        """,
        shortest=5,
        trigger=True,
    ),
    Case(
        # Agent 0 walks from a lift whose destination is deadly onto a safe lift of the same group
        # while the button is pressed: only the lift it ends the walk on moves it.
        "walk-from-deadly-lift-onto-safe-lift",
        """
        S0 TU0 TU0 B0 S1
        .  .   .   .  X
        ;
        .  .   X   .  .
        .  L1N .   .  .
        """,
        shortest=4,
        trigger=True,
    ),
    Case(
        # A box sits on the destination: agent 2 must push it away before the lift can work.
        "box-on-destination",
        """
        .  .   .  .  .
        S0 TU0 B0 S1 X
        .  .   .  .  .
        ;
        .  S2  .  .  X
        .  #   .  .  .
        X  .   .  .  .
        """,
        shortest=5,
        trigger=True,
    ),
    Case(
        "push-box-off-lift-and-ride",
        """
        S0 # TU0 . .
        .  . B0  S1 X
        ;
        .  . X   .  .
        .  . .   .  .
        """,
        shortest=4,
        trigger=True,
    ),
    Case(
        # Pushing a box onto the lift jams it until someone pushes it off again.
        "box-pushed-onto-lift",
        """
        S0 # TU0 B0 S1
        .  . .   .  X
        ;
        .  . X   .  .
        .  . .   .  .
        """,
        shortest=6,
        trigger=True,
    ),
    Case(
        "lift-onto-void",
        """
        S0 TU0 B0 S1 X
        ;
        X  V   .  .  .
        """,
        shortest=None,
    ),
    Case(
        "lift-onto-wall",
        """
        S0 TU0 B0 S1 X
        ;
        X  @   .  .  .
        """,
        shortest=None,
    ),
    Case(
        "lift-out-of-the-world",
        """
        X  .   .  .  .
        ;
        S0 TU0 B0 S1 X
        """,
        shortest=None,
    ),
    Case(
        "ride-onto-a-gem",
        """
        S0 TU0 B0 S1 X
        ;
        X  G   .  .  .
        """,
        shortest=4,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        # Agent 2 must leave the destination in the very step agent 0 is lifted onto it.
        "lift-onto-a-cell-being-left",
        """
        S0 TU0 B0 S1 X
        ;
        X  S2  .  .  X
        """,
        shortest=4,
        trigger=True,
    ),
    Case(
        # Agent 0 rides up, then presses a button upstairs to lift agent 1.
        "press-upstairs-for-the-next-rider",
        """
        S0 TU0 B0 TU1 S1
        ;
        X  .   B1 .   X
        """,
        shortest=7,
        trigger=True,
    ),
    Case(
        # Both riders need a lift towards the middle floor, from opposite directions.
        "converging-lifts",
        """
        S0 TU0 B0 S1 X
        ;
        X  .   .  .  X
        ;
        S2 TD0 B0 .  .
        """,
        shortest=5,
        trigger=True,
    ),
    Case(
        # The rider lands on a lift of the next floor, and rides it again with another press.
        "chained-lifts",
        """
        S0 TU0 B0 S1 X
        ;
        .  TU1 B1 S2 X
        ;
        X  .   .  .  .
        """,
        shortest=5,
        trigger=True,
    ),
    Case(
        # Agent 0 may land on a lift of group 1 while button 1 lifts agent 2 elsewhere.
        "land-on-a-pulsed-lift",
        """
        S0 TU0 B0  S1  X
        .  .   .   .   X
        ;
        .  TU1 B1  TU1 S2
        .  X   S3  .   .
        ;
        X  X   .   X   .
        .  .   .   .   .
        """,
        shortest=4,
        trigger=True,
    ),
    Case(
        # The rider blocks its own beam on the lift; once lifted, the beam reopens downstream.
        "lifted-off-own-beam",
        """
        L0E TU0 .  .  X
        S0  .   B0 S1 X
        ;
        .   .   X  .  .
        .   .   .  .  .
        """,
        shortest=4,
        trigger=True,
    ),
    Case(
        # The lift is the first tile of a foreign beam: nobody else can stand on it.
        "lift-on-the-first-beam-tile",
        """
        L0E TU0 B0 .  X
        S0  .   S1 .  .
        ;
        .   X   .  .  .
        .   .   .  .  .
        """,
        shortest=None,
    ),
    Case(
        # Agent 1 presses from under its own beam, which it blocks for the rider downstream.
        "press-under-own-beam",
        """
        S0 TU0 B0 S1 L1W
        .  .   .  .  X
        ;
        X  .   .  .  .
        .  .   .  .  .
        """,
        shortest=5,
        trigger=True,
    ),
    Case(
        # The rider owns the beam over the button, and blocks it for the presser while waiting.
        "press-under-foreign-beam",
        """
        S1 TU0 B0 S0 L0W
        .  .   .  .  X
        ;
        X  .   .  .  .
        .  .   .  .  .
        """,
        shortest=5,
        trigger=True,
    ),
    Case(
        # Boxes and a laser upstairs, with the rider pushing a box after landing.
        "push-box-upstairs",
        """
        S0  TU0 B0 S1 X
        .   .   .  .  .
        ;
        .   .   #  .  .
        L1E .   .  .  X
        """,
        shortest=8,
        trigger=True,
    ),
]


@pytest.mark.parametrize("case", CASES, ids=lambda case: case.id)
def test_tricky_lift_worlds(case: Case):
    check(case)


# Random worlds (two or three floors, with lifts, buttons, boxes, lasers, voids, walls, gems and
# colour restrictions) whose shortest plan presses a button, or that are unsolvable.
RANDOM_CASES = [
    Case(
        "random-15",
        """
        . . L0W B1A1 X
        . S1 . S2 TU0
        ;
        V X V TD0 .
        B0 S0 X # .
        """,
        shortest=5,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-17",
        """
        TU1 . . L0W
        . # . .
        . S0 . .
        ;
        . . X TU1
        . . . .
        . . @ .
        ;
        S1 @ . L0N
        TD1A1 . X .
        . . . B1A1
        """,
        shortest=8,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-22",
        """
        X B1A0 B0
        S2 S1 TU0
        ;
        S0 # X
        TD0 X TD1
        """,
        shortest=5,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-25",
        """
        # S0 TU0
        TU0 B0 S1
        ;
        X L1E TD1
        B0 @ X
        """,
        shortest=7,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-55",
        """
        S0 L0W L1E
        X . X
        ;
        S1 S2 X
        TD1 TD0 B0
        """,
        shortest=4,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-57",
        """
        . . S2 S0 B1
        . TU1 . L1S .
        . . . . X
        ;
        . . . . X
        . . S1 . .
        X . . L2E .
        """,
        shortest=4,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-68",
        """
        . . . . S0
        . . . . X
        . . . . .
        ;
        . . . # .
        S2 . B0 . .
        TU1 . . . #
        ;
        . . . . .
        S1 . . . X
        . . B1 . X
        """,
        shortest=8,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-175",
        """
        S2 TU1A2 S0
        B0 TU1 X
        ;
        X S1 B1
        L2N X TD0
        """,
        shortest=4,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-213",
        """
        . X G
        B1A1 G L0E
        . X .
        ;
        TD1 S0 TD0
        TD1 . .
        B1 L1S S1
        """,
        shortest=8,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-215",
        """
        . . .
        X . TU1
        . . .
        ;
        X . TD1A0
        B1A0 S1 S0
        @ B0A0 TD1
        """,
        shortest=7,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-234",
        """
        X TU0 TU1
        # # B1
        S1 TU1A2 S2
        ;
        X B0 S0
        L0S . X
        . . .
        """,
        shortest=7,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-1015",
        """
        . . S0 S2
        X B1 G L2S
        . . TU1 .
        ;
        . . X .
        S1 . . #
        # . . X
        """,
        shortest=5,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-1055",
        """
        S1 B1 X S0
        TU1 TU0 . .
        ;
        X S2 B0 L1N
        # X # .
        """,
        shortest=4,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-1062",
        """
        G B1 . X
        S2 S0 TU1 L1S
        ;
        L2S TD0 . X
        B0 . S1 X
        """,
        shortest=5,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-1078",
        """
        . . TU1A1 .
        . S1 . .
        ;
        . X . B1
        TD0 . X S0
        """,
        shortest=4,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-1102",
        """
        TU0A2 # S2
        TU1 . .
        . . .
        ;
        . TU1 .
        S0 . .
        B1A0 . X
        ;
        S1 . .
        # . X
        . X .
        """,
        shortest=7,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-1108",
        """
        S0 # . .
        . . B1 X
        X . . .
        ;
        . . L2N X
        # S1 . S2
        . TD1A2 . .
        """,
        shortest=5,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-1111",
        """
        X . B1 X
        . . X .
        ;
        # S1 S0 G
        B0A2 TD0 TD1 S2
        """,
        shortest=8,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-1148",
        """
        . . X X .
        . . V . B1
        . S0 . TU0 @
        ;
        . . . S1 TD0
        B0 . . S2 #
        X . . L0S .
        """,
        shortest=5,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-1160",
        """
        . B1A1 S1 .
        S0 . X TU1
        ;
        # . X TD0
        . L1S . .
        """,
        shortest=8,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-1162",
        """
        X . B0
        . L0W TU0
        ;
        . S0 B1A1
        TD1 S1 X
        """,
        shortest=5,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-2051",
        """
        L1W . . . .
        . . . TU1 S1
        . . . . .
        ;
        X . . . .
        X . B1A0 . .
        S0 . . B1A1 .
        """,
        shortest=8,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-2072",
        """
        S2 X S0 X
        . TU1 B1 L1N
        ;
        TD1A1 L1E # TD1
        X . S1 G
        """,
        shortest=4,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-2127",
        """
        X B0 .
        . TU0 #
        X . S1
        ;
        TD1 B1 S0
        X V TD1
        S2 . .
        """,
        shortest=6,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-2135",
        """
        S2 TU0 .
        . . .
        ;
        X . .
        G . .
        ;
        S0 B0 X
        B1A2 S1 X
        """,
        shortest=3,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-2157",
        """
        TU0 X S0 .
        B1 . . .
        . # . S1
        ;
        . X . .
        . . . .
        . . # @
        ;
        TD0A2 . . .
        X . . S2
        . . . B0
        """,
        shortest=8,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-2170",
        """
        . . B1A0
        . . TU0
        # S0 L0N
        ;
        . # L0N
        . S1 X
        B0 . X
        """,
        shortest=5,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-2192",
        """
        B0 X X
        S1 . B0
        ;
        V G TD0
        S0 . #
        """,
        shortest=4,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-2229",
        """
        B1 S2 X
        # X B0
        ;
        # S1 TD0
        S0 TD1 X
        """,
        shortest=4,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-3046",
        """
        S0 X .
        X . #
        . B0A0 L1S
        ;
        L0N B0 S1
        . X TD1
        S2 TD0 .
        """,
        shortest=4,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-3047",
        """
        # L2N S0 .
        S2 . TU0 .
        B0 . X G
        ;
        B1A0 S1 . TD1
        . # . .
        X . . X
        """,
        shortest=4,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-3060",
        """
        . . X @ TU0A1
        S1 . . . X
        ;
        . . . . X
        B0 S2 S0 TD0 .
        """,
        shortest=7,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-3068",
        """
        TU1 . X
        TU1 . S0
        . B1 .
        ;
        . . G
        G TD1 S1
        . X .
        """,
        shortest=6,
        trigger=False,
        collect_gems=True,
    ),
    Case(
        "random-3076",
        """
        . X B0A0
        . . .
        S0 X S1
        ;
        L0N S2 TU0A2
        . # L0E
        . . #
        ;
        . . .
        . TD0 .
        . X .
        """,
        shortest=8,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-3091",
        """
        TU0 TU0 S2 X
        TU1A0 S1 B1 B0
        ;
        X . . #
        . S0 X .
        """,
        shortest=4,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-3101",
        """
        . X X B1 .
        . . # . S0
        ;
        # . TD1 . B0A1
        S1 . TD0 V G
        """,
        shortest=7,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-3103",
        """
        TU1 TU0A2 .
        X B0 #
        S0 . S2
        ;
        X X .
        . S1 TD1A2
        . # B1
        """,
        shortest=6,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-3108",
        """
        S0 V . #
        . . TU0A1 .
        . . X S1
        ;
        B0 B0 X .
        X . . @
        . . S2 .
        """,
        shortest=6,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-3117",
        """
        S2 # X TU1
        B0A1 . # S0
        ;
        X V TD0A1 X
        . . S1 B1
        """,
        shortest=6,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-3170",
        """
        # . . . #
        . . . B1A1 S0
        . . . TU1 .
        ;
        . . . B1 S1
        . TD0A1 . . .
        TD1 . X X .
        """,
        shortest=5,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-3173",
        """
        . . .
        X . TU0A0
        S2 TU1 X
        ;
        . # B0
        S0 TD0 G
        S1 X .
        """,
        shortest=7,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-3190",
        """
        . . . S1
        . . X X
        ;
        B1 . TD1 TD1
        S2 # S0 X
        """,
        shortest=6,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-3237",
        """
        TU1 # .
        . TU1A1 S0
        X X .
        ;
        . X .
        . . S2
        B0 TD0 S1
        """,
        shortest=6,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-3242",
        """
        . . . . TU1
        . # . S1 .
        . B1A0 S0 . X
        ;
        . . V . .
        . . @ . TD0
        . X B0A0 . TD0
        """,
        shortest=7,
        trigger=True,
        collect_gems=True,
    ),
    Case(
        "random-3247",
        """
        . . . V
        . X B1 .
        . L1S S1 X
        ;
        . TD0A2 X TD1
        L1W S0 S2 .
        TD1 B1 G .
        """,
        shortest=4,
        trigger=True,
        collect_gems=False,
    ),
    Case(
        "random-5002",
        """
        L0S . S0
        . . .
        . # .
        ;
        . TU0 .
        . V .
        B1A0 . .
        ;
        . . .
        G . .
        . L0S X
        """,
        shortest=None,
        max_length=8,
        trigger=False,
        collect_gems=False,
    ),
    Case(
        "random-5004",
        """
        . . . S0
        . . . .
        . S1 X #
        ;
        X L0E X .
        S2 . . .
        TD0A1 . . B0
        """,
        shortest=None,
        max_length=8,
        trigger=False,
        collect_gems=False,
    ),
    Case(
        "random-5008",
        """
        . B1A0 TU0A0 B1A0 S0
        . # . . L0W
        ;
        . . . L0N .
        V . X X S1
        """,
        shortest=None,
        max_length=8,
        trigger=False,
        collect_gems=False,
    ),
    Case(
        "random-5012",
        """
        . . # L0W
        S0 S1 S2 TU1A0
        ;
        . X # X
        B0 L0E TD0A1 X
        """,
        shortest=None,
        max_length=8,
        trigger=False,
        collect_gems=False,
    ),
    Case(
        "random-5014",
        """
        TU1A2 TU0 X #
        TU1 X S1 #
        ;
        V . S0 X
        S2 . L0S B1
        """,
        shortest=None,
        max_length=8,
        trigger=False,
        collect_gems=False,
    ),
    Case(
        "random-5020",
        """
        # . . S1 .
        X . . S2 B1A0
        . . . TU1 L2S
        ;
        X . . . .
        . . . . X
        . L1N V # S0
        """,
        shortest=None,
        max_length=8,
        trigger=False,
        collect_gems=False,
    ),
    Case(
        "random-5021",
        """
        TU1 . . B0 TU1
        . . . . V
        # . . . .
        ;
        . . # . S0
        . . . . .
        . L0N . . B0A0
        ;
        . . . . .
        . V . . .
        . . X TD1A0 .
        """,
        shortest=None,
        max_length=8,
        trigger=False,
        collect_gems=False,
    ),
    Case(
        "random-5026",
        """
        L1N S0 . . .
        V B1 TU1A1 TU1A2 S2
        ;
        . . L1W . #
        X . # X S1
        ;
        X . . . .
        . . . . .
        """,
        shortest=None,
        max_length=8,
        trigger=False,
        collect_gems=False,
    ),
]


@pytest.mark.parametrize("case", RANDOM_CASES, ids=lambda case: case.id)
def test_random_lift_worlds(case: Case):
    check(case)


def test_solve_world_with_layers_and_no_lift():
    world = "S0 . X\n;\n. . ."
    check(Case("layers-without-lift", world, shortest=2))


def test_cooperation_modes_raise_on_world_with_lifts():
    with pytest.raises(NotImplementedError, match="lifts"):
        lle.solve(World(CASES[0].map), 10, mode="no-cooperation")


def test_characterize_raises_on_world_with_lifts():
    with pytest.raises(NotImplementedError, match="lifts"):
        lle.characterize(World(CASES[0].map), 10)
