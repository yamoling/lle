"""SAT solving of worlds with lifts.

Every plan returned by the solver is replayed in `World`: each action must be available, no agent
may die and every agent must arrive.
"""

from collections.abc import Sequence

import lle
import pytest
from lle import EventType, World

# Agent 1 presses the button while agent 0 walks onto the lift and rides it to the upper exit.
RIDE = """
S0 TU0 B0 S1 X
;
X  .   .  .  .
"""


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


def test_solve_rides_a_lift():
    plan = lle.solve(World(RIDE), 10, path_length=4)
    assert plan is not None
    assert lle.Action.TRIGGER in [action for joint_action in plan for action in joint_action]
    assert_plan_is_valid(RIDE, plan)


def test_lift_shortest_plan_needs_the_press():
    assert lle.solve(World(RIDE), 10, path_length=3) is None


def test_lift_refuses_a_rider_of_another_colour():
    world = "S0 TU0A1 B0 S1 X\n;\nX . . . ."
    assert all(lle.solve(World(world), 10, path_length=n) is None for n in range(1, 11))


def test_solve_pushes_a_box_off_a_lift_and_rides_it():
    # Agent 0 pushes the box onto the lift, then pushes it off while riding the lift up.
    world = """
    S0 # TU0 .  .
    .  . B0  S1 X
    ;
    .  . X   .  .
    .  . .   .  .
    """
    assert lle.solve(World(world), 8, path_length=3) is None
    plan = lle.solve(World(world), 8, path_length=4)
    assert plan is not None
    assert_plan_is_valid(world, plan)


def test_solve_world_with_layers_and_no_lift():
    world = "S0 . X\n;\n. . ."
    plan = lle.solve(World(world), 5, path_length=2)
    assert plan is not None
    assert_plan_is_valid(world, plan)


def test_cooperation_modes_raise_on_world_with_lifts():
    with pytest.raises(NotImplementedError, match="lifts"):
        lle.solve(World(RIDE), 10, mode="no-cooperation")


def test_characterize_raises_on_world_with_lifts():
    with pytest.raises(NotImplementedError, match="lifts"):
        lle.characterize(World(RIDE), 10)
