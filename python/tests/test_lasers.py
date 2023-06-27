from laser_env.tiles import Laser
from laser_env.world import World
from laser_env.actions import Action


def test_laser_blocked_on_reset():
    world = World("tests/maps/laser_on_spawn")
    world.reset()
    assert not any(agent.is_dead for agent in world.agents)
    laser = world[3, 2]
    assert isinstance(laser, Laser)
    assert laser.is_off


def test_facing_lasers():
    world = World("tests/maps/facing_lasers")
    world.reset()
    _, done = world.step([Action.WEST, Action.WEST])
    assert not done
    assert not any(agent.is_dead for agent in world.agents)
    laser = world[2, 2]
    assert isinstance(laser, Laser)
    assert laser.is_off


def test_reset_with_laser():
    """Check that reset() indeed removes the agent from the laser's tile"""
    world = World("tests/maps/5x5_laser_1agent")
    world.reset()
    tile = world[2, 2]
    assert tile.is_empty
    world.step([Action.WEST])
    assert tile.is_occupied
    world.reset()
    assert tile.is_empty


def test_not_killed():
    world = World("tests/maps/5x5_laser_1agent")
    world.reset()
    _, done = world.step([Action.WEST])
    assert not done
    assert not any(agent.is_dead for agent in world.agents)
    _, done = world.step([Action.WEST])
    assert done


def test_kill_agent_walk_into():
    world = World("tests/maps/5x5_laser_2agents")
    world.reset()
    _, done = world.step([Action.NOOP, Action.WEST])
    assert done
    assert any(agent.is_dead for agent in world.agents)


def test_block_laser():
    world = World("tests/maps/5x5_laser_2agents")
    world.reset()

    # Block the laser while the other Action.NOOPs
    _, done = world.step([Action.WEST, Action.NOOP])
    assert not done
    assert not any(agent.is_dead for agent in world.agents)

    # Move the other in the ray
    _, done = world.step([Action.NOOP, Action.WEST])
    assert not done
    assert not any(agent.is_dead for agent in world.agents)


def test_block_laser2():
    """
    Walk in the laser with both agents simultaneously. The former should block the laser for the latter
    and none should die.
    """
    world = World("tests/maps/5x5_laser_2agents")
    world.reset()

    # Block the laser while the other Action.NOOPs
    _, done = world.step([Action.WEST, Action.WEST])
    assert not done
    assert not any(agent.is_dead for agent in world.agents)


def test_block_laser3():
    """
    Walk in the laser with both agents simultaneously but in the
    wrong order: the first hit by the ray should die.
    """
    world = World("tests/maps/5x5_laser_2agents_bis")
    world.reset()

    _, done = world.step([Action.WEST, Action.WEST])
    assert done
    assert any(agent.is_dead for agent in world.agents)


def test_kill_leave_and_stay():
    world = World("tests/maps/5x5_laser_2agents")
    world.reset()
    _, done = world.step([Action.WEST, Action.WEST])
    assert not done
    assert not any(agent.is_dead for agent in world.agents)

    _, done = world.step([Action.EAST, Action.NOOP])
    assert done
    assert any(agent.is_dead for agent in world.agents)


def test_laser_over_void():
    world = World("tests/maps/5x5_laser_over_void")
    assert isinstance(world[1, 2], Laser)
    assert isinstance(world[2, 2], Laser)
    assert isinstance(world[3, 2], Laser)
    assert not isinstance(world[4, 2], Laser)


def test_end_of_world_stops_laser():
    """The environment should not crash when there is no wall to stop a laser"""
    World("tests/maps/5x5_laser_no_wall")


def test_laser_source_blocks_laser_beam():
    world = World("tests/maps/5x5_laser_blocked_by_laser_source")
    # Check South laser source
    assert isinstance(world[1, 2], Laser)
    assert not isinstance(world[3, 2], Laser)

    # Check Action.WEST laser source
    assert isinstance(world[2, 1], Laser)


def test_laser_activates_when_blocking():
    """In the following map, if agent 1 is in (0, 2) and agent 2 is in (0, 3), agent 1 is blocking the laser.
    When agent 2 leaves the tile and goes to the right, the laser should NOT activate."""
    world = World.from_str(
        """
    G L0E F . F
    G G . . L1W
    @ S0 . . @
    . @ . . .
    S1 G . . G"""
    )
    world.reset()
    laser_tile = world[0, 3]
    assert isinstance(laser_tile, Laser)
    assert laser_tile.is_on

    world.force_state([(0, 2), (0, 3)], [False] * 5)
    assert laser_tile.is_off

    world.step([Action.NOOP, Action.EAST])
    assert laser_tile.is_off
