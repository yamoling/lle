"""Discrete-plan oracle (design doc §11, round A §5).

The SAT solver finds a plan for the discrete `World`. Each discrete action becomes a unit velocity
held for exactly one cell of travel, `round(1 / (max_speed * dt))` ticks (10 with the defaults), and
the continuous world must reach the same outcome: every agent exits, every gem is collected and
nobody dies.

The plans are the shortest ones that collect every gem (`Solver.find_shortest(collect_gems=True)`),
so that they contain no detour. With the default physics, all six built-in levels pass: agents walk
along cell centres, two agents following each other stay 1 cell apart (more than 2r = 0.6), agents
crossing at a corner stay at least sqrt(0.5) ≈ 0.71 apart, and a blocker enters (or leaves) a beam
in the very tick its protégé does. A level that legitimately fails because of beam timing must be
marked `xfail` with the reason, rather than loosening the physics. The solver takes about 2 s for
the six levels in a debug build; each plan is computed once and shared by the tests.

The same replay also checks the `LAYERED` observation at `res = 1` against LLE's `Layered` after
every discrete action, when the agents stand at cell centres. Two differences are allowed, and
nothing else:
- the cell of an agent standing in a beam of its own colour is lit in the laser channel of CLLE
  (the beam reaches into the cell up to the disc), while LLE turns the beam off from that cell on;
- an arrived agent stops as soon as its centre enters the exit, near the edge of the cell, so its
  disc also overlaps the cell it came from, while LLE puts it at the centre of the exit.
"""

import functools
import math

import lle
import numpy as np
import pytest
from lle.continuous import ContinuousWorld, PhysicsParams
from lle.observations import Layered
from lle.solver import Solver

from .common import velocity_of

T_MAX = 40


def ticks_per_cell(params: PhysicsParams) -> int:
    """The number of ticks to travel one cell at full speed."""
    return round(1 / (params.max_speed * params.dt))


@functools.cache
def shortest_plan(level: int):
    """The shortest discrete plan of the built-in level `level` that collects every gem."""
    plan = Solver(lle.World.level(level), T_MAX).find_shortest(collect_gems=True)
    assert plan is not None, f"no discrete plan within {T_MAX} steps"
    return plan


@pytest.mark.parametrize("level", [1, 2, 3, 4, 5, 6])
def test_discrete_plan_solves_the_continuous_level(level: int):
    plan = shortest_plan(level)

    world = ContinuousWorld.level(level)
    world.reset()
    n_ticks = ticks_per_cell(world.params)
    deaths = []
    for t, joint_action in enumerate(plan):
        outcome = world.step([velocity_of(a) for a in joint_action], n_ticks)
        deaths += [(t, e.agent_id) for e in outcome.events if e.event_type == lle.EventType.AGENT_DIED]
    assert deaths == [], f"deaths (step, agent): {deaths}"
    assert all(world.agents_alive)
    assert all(world.agents_arrived)
    assert all(world.gems_collected)


def allowed_difference(world: ContinuousWorld, channel: int, i: int, j: int) -> bool:
    """
    Whether CLLE may light the pixel `(channel, i, j)` of `layered(1)` where LLE does not (see the
    module docstring): the cell of an alive agent in the laser channel of its colour, or a cell
    overlapped by the disc of an arrived agent in the agent channel of its colour.

    @ai-generated
    """
    n_colours = world.n_colours
    r = world.params.agent_radius
    for agent, colour in enumerate(world.agents_colours):
        if not world.agents_alive[agent]:
            continue
        x, y = world.agents_positions[agent]
        if channel == n_colours + colour and tuple(world.agents_cells[agent])[:2] == (i, j):
            return True
        if channel == colour and world.agents_arrived[agent]:
            # The distance from the centre of the disc to the cell (i, j).
            dx = max(j - x, 0.0, x - (j + 1))
            dy = max(i - y, 0.0, y - (i + 1))
            if math.hypot(dx, dy) < r:
                return True
    return False


@pytest.mark.parametrize("level", [1, 2, 3, 4, 5, 6])
def test_layered_at_res_1_matches_lle_along_the_discrete_plan(level: int):
    discrete = lle.World.level(level)
    discrete.reset()
    world = ContinuousWorld.level(level)
    world.reset()
    n_ticks = ticks_per_cell(world.params)
    n_allowed = 0
    for t, joint_action in enumerate(shortest_plan(level)):
        discrete.step(list(joint_action))
        world.step([velocity_of(a) for a in joint_action], n_ticks)
        expected = Layered(discrete).observe()[0][..., 0]
        actual = world.layered(1)
        for channel, i, j in np.argwhere(actual != expected):
            assert actual[channel, i, j] == 1.0 and expected[channel, i, j] == 0.0, (t, channel, i, j)
            assert allowed_difference(world, int(channel), int(i), int(j)), (t, channel, i, j)
            n_allowed += 1
    # Every plan ends with the agents arrived, so the second difference always shows up.
    assert n_allowed > 0
