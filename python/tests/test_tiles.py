"""
- Test source.enable() and source.disable() methods
- test assignments for source.is_enabled and source.is_disabled
- test assignments for source.agent_id
- test that source.direction and source.laser_id are read-only
- test gem.collect() method
"""

from lle import Action, World


def test_lift_and_button_agent_under_a_laser_beam():
    # Agent 1 stands on the button under its own beam, which also covers the lift.
    world = World("S0 TU0 B0 S1 L1W\n.  .   .  .  X\n;\nX  .   .  .  .\n.  .   .  .  .")
    world.reset()
    world.step([Action.EAST, Action.WEST])
    assert world.buttons[0].agent == 1
    assert world.lifts[0].agent == 0
