import lle
import pytest
from lle import World

from ..pending import call_or_xfail_unimplemented
from ..world_layouts import LEVEL_1, LEVEL_2, LEVEL_3, LEVEL_4, LEVEL_5, LEVEL_6, Layout


def test_time_dependent_threshold():
    """A mutual-free plan appears when the laser-free detour opens at t=13."""
    world = World("""
     S0 S1 . . .
    L0E  . . @ .
    L1E  . . @ .
      X  X . @ .
      .  . . . .
    """)
    threshold = 13

    for t_max in range(threshold):
        if lle.solve(world, t_max) is None:
            continue
        plan = call_or_xfail_unimplemented(
            lle.solve,
            world,
            t_max,
            mode="no-mutual",
        )
        assert plan is None, f"expected mutual help at t={t_max}"

    plan = call_or_xfail_unimplemented(
        lle.solve,
        world,
        threshold,
        mode="no-mutual",
    )
    assert plan is not None

    world.reset()
    for joint_action in plan:
        world.step(joint_action)
    assert all(agent.is_alive and agent.has_arrived for agent in world.agents)


@pytest.mark.parametrize(
    ("layout", "t_max", "plan_expected"),
    [
        pytest.param(LEVEL_1, 10, True, id="level-1"),
        pytest.param(LEVEL_2, 10, True, id="level-2"),
        pytest.param(LEVEL_3, 10, True, id="level-3"),
        pytest.param(LEVEL_4, 10, False, id="level-4"),
        pytest.param(LEVEL_5, 19, True, id="level-5"),
        pytest.param(LEVEL_6, 21, False, id="level-6"),
    ],
)
def test_solve_standard_levels_without_mutual_cooperation(layout: Layout, t_max: int, plan_expected: bool):
    """Built-in levels retain their declared mutual-free solvability."""
    plan = call_or_xfail_unimplemented(
        lle.solve,
        layout.world(),
        t_max,
        mode="no-mutual",
    )
    assert (plan is not None) is plan_expected
