import lle
from lle import World
from lle.characterization import profile_plan

# Agent 1 crosses the beam of colour 0 downstream of the box while agent 0 stands upstream of it.
BOX_PROTECTED = """
@   S0 @ X  @
L0E .  # .  @
@   X  @ S1 @
"""

# Same world without the box: agent 0 must block the beam for agent 1.
OWNER_PROTECTED = BOX_PROTECTED.replace("#", ".")


def test_beam_blocked_by_box_is_not_help():
    world = World(BOX_PROTECTED)
    plan = lle.solve(world, 10, path_length=2)
    assert plan is not None
    assert profile_plan(world, plan).graph.is_empty
    assert lle.solve(world, 10, path_length=2, mode="no-cooperation") is not None
    assert not lle.is_cooperative(world, 10)


def test_beam_blocked_by_owner_is_help():
    world = World(OWNER_PROTECTED)
    assert lle.solve(world, 10, mode="no-cooperation") is None
    assert lle.is_cooperative(world, 10)


def test_characterize_world_with_boxes():
    lle.characterize(World(BOX_PROTECTED), 10)
