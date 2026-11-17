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
the six levels in a debug build.
"""

import lle
import pytest
from lle.continuous import ContinuousWorld, PhysicsParams
from lle.solver import Solver

from .common import velocity_of

T_MAX = 40


def ticks_per_cell(params: PhysicsParams) -> int:
    """The number of ticks to travel one cell at full speed."""
    return round(1 / (params.max_speed * params.dt))


@pytest.mark.parametrize("level", [1, 2, 3, 4, 5, 6])
def test_discrete_plan_solves_the_continuous_level(level: int):
    discrete = lle.World.level(level)
    plan = Solver(discrete, T_MAX).find_shortest(collect_gems=True)
    assert plan is not None, f"no discrete plan within {T_MAX} steps"

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
