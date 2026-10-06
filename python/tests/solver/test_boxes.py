"""SAT solving of worlds with movable boxes.

Every plan returned by the solver is replayed in `World`: each action must be available, no agent
may die and every agent must arrive.
"""

from collections.abc import Sequence
from dataclasses import dataclass

import lle
import pytest
from lle import EventType, World
from lle.solver import Solver

T_MAX = 9

# Agent 1 is parked in this walled pocket, next to two exits, so that laser colour 1 has an owner
# that never interferes.
POCKET = "\n@ @ @ S1 X X"


def assert_plan_is_valid(map_str: str, plan: Sequence[Sequence[lle.Action]]):
    world = World(map_str)
    world.reset()
    for step, joint_action in enumerate(plan):
        available = world.available_actions()
        for agent, action in enumerate(joint_action):
            assert action in available[agent], f"step {step}: {action} unavailable for agent {agent}"
        events = world.step(list(joint_action))
        assert all(event.event_type != EventType.AGENT_DIED for event in events), f"an agent died at step {step}"
    assert all(agent.has_arrived for agent in world.agents)


@dataclass(frozen=True)
class BoxCase:
    id: str
    map: str
    shortest: int | None


CASES = [
    BoxCase(
        "push-along-corridor",
        """
        S0 B . .
        @  @ X @
        """,
        3,
    ),
    BoxCase(
        "box-blocks-beam",
        """
        L1S . . . @ @
        .   B S0 . @ @
        .   . . . @ @
        X   @ @ @ @ @"""
        + POCKET,
        4,
    ),
    BoxCase(
        "beam-unblockable-without-box",
        """
        L1S . . . @ @
        .   . S0 . @ @
        .   . . . @ @
        X   @ @ @ @ @"""
        + POCKET,
        None,
    ),
    BoxCase(
        "box-destroyed-in-void-does-not-block",
        """
        L1S . . . @ @
        V   B S0 . @ @
        .   . . . @ @
        X   @ @ @ @ @"""
        + POCKET,
        None,
    ),
    BoxCase(
        "box-pushed-along-beam-protects-pusher",
        """
        S0 . B . L1W @
        @  @ X @ @   @"""
        + POCKET,
        3,
    ),
    BoxCase(
        # The box could only be pushed onto the laser source or off the grid: it never moves, and in
        # particular never slides into the beam or duplicates itself onto it.
        "immovable-box-neither-moves-nor-duplicates",
        """
        S0  . . @ @ @
        B   . . @ @ @
        L1E . . @ @ @
        @   @ X @ @ @"""
        + POCKET,
        None,
    ),
    BoxCase(
        "box-cannot-be-pushed-into-wall",
        """
        S0 B @
        @  X @
        """,
        None,
    ),
    BoxCase(
        "box-cannot-be-pushed-off-grid",
        """
        S0 B X
        @  @ @
        """,
        None,
    ),
    BoxCase(
        "box-cannot-be-pushed-into-box",
        """
        S0 B B . .
        @  @ X @ @
        """,
        None,
    ),
    BoxCase(
        "push-box-into-void-to-clear-path",
        """
        .  . @ @
        S0 B . X
        @  V @ @
        """,
        5,
    ),
    BoxCase(
        "same-path-without-void-is-blocked",
        """
        .  . @ @
        S0 B . X
        @  @ @ @
        """,
        None,
    ),
    BoxCase(
        # Agent 0 must wait one step: the box cannot be pushed onto the cell agent 1 is leaving.
        "box-cannot-follow-leaving-agent",
        """
        S0 B S1 X
        @  . .  X
        @  @ @  X
        """,
        5,
    ),
    BoxCase(
        # Agent 1 would cross the colour-2 beam while agent 0 pushes the box onto the void under
        # it: the box is destroyed at once and never blocks the beam.
        "box-falling-into-void-under-beam-does-not-block",
        """
        X   S0 .  .
        .   B  S1 .
        L2E V  .  .
        @   @  X  @
        """,
        None,
    ),
    BoxCase(
        # No agent has colour 2: only a box can block that beam.
        "box-blocks-beam-of-colour-without-agent",
        """
        L2S . .
        .   B S0
        .   . .
        X   @ @
        """,
        4,
    ),
    BoxCase(
        "beam-of-colour-without-agent-is-unblockable",
        """
        L2S . .
        .   . S0
        .   . .
        X   @ @
        """,
        None,
    ),
    BoxCase(
        # Agent 0 has colour 1: it cannot block the colour-0 beam, but the box can.
        "laser-colour-differs-from-agent-index",
        """
        L0S . .
        .   B S1
        .   . .
        X   @ @
        """,
        4,
    ),
    BoxCase(
        "agent-index-does-not-block-beam-of-same-number",
        """
        L0S . .
        .   . S1
        .   . .
        X   @ @
        """,
        None,
    ),
]


@pytest.mark.parametrize("case", CASES, ids=lambda case: case.id)
def test_box_case(case: BoxCase):
    plan = Solver(World(case.map), T_MAX).find_shortest()
    assert (None if plan is None else len(plan)) == case.shortest
    if plan is not None:
        assert_plan_is_valid(case.map, plan)


@pytest.mark.parametrize("mode", ["no-cooperation", "no-mutual", "no-sequence"])
def test_cooperation_modes_are_not_supported_with_boxes(mode: str):
    solver = Solver(World("S0 B . .\n@ @ X @"), T_MAX)
    with pytest.raises(NotImplementedError, match="movable boxes"):
        solver.solve(mode=mode)
    with pytest.raises(NotImplementedError, match="movable boxes"):
        solver.find_shortest(mode=mode)
