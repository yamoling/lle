from threading import Thread
import pytest
from copy import deepcopy

from lle import World, WorldState, Action, ParsingError, InvalidActionError, EventType, InvalidWorldStateError


def test_world_tiles():
    w = World("S0 . X")
    assert w.start_pos == [(0, 0)]
    assert w.exit_pos == [(0, 2)]


def test_available_actions():
    world = World(
        """
@ @ L0S @  @
@ .  .  .  @
@ X  .  S0 @
@ X  .  S1 @
@ @  @  @  @
"""
    )
    world.reset()
    available_actions = world.available_actions()
    # Agent 0
    expected = [[Action.NORTH, Action.WEST, Action.STAY], [Action.WEST, Action.STAY]]
    for e, available in zip(expected, available_actions):
        for a in available:
            assert a in e, f"Action {a} not in {e}"


def test_parse_wrong_worlds():
    # Not enough finish tiles for all the agents
    with pytest.raises(ParsingError):
        World(
            """
            @ @  @ @
            @ S0 . @
            @ .  . @
            @ @  @ @"""
        )

    # Zero agent in the environment
    with pytest.raises(ParsingError):
        World("X G")


def test_world_move():
    world = World(
        """S0 X . .
.  . . .
.  . . ."""
    )
    world.reset()
    world.step([Action.SOUTH])
    world.step([Action.EAST])
    world.step([Action.NORTH])


def test_world_agents():
    world = World(
        """
                  S0 S1 S2
                  X  X  X"""
    )
    world.reset()
    assert not any(a.is_dead for a in world.agents)
    assert all(a.is_alive for a in world.agents)


def test_walk_into_wall():
    world = World(
        """@ @ @  @ @ @
@ . .  . . @
@ . S0 . . @
@ . .  X . @
@ @ @  @ @ @"""
    )
    world.reset()
    world.step([Action.SOUTH])
    with pytest.raises(InvalidActionError):
        world.step([Action.SOUTH])


def test_gem_collected_and_agent_died():
    world = World(
        """
S0  G  X
S1 L1N X"""
    )
    world.reset()
    events = world.step([Action.EAST, Action.STAY])
    assert len(events) == 1
    assert events[0].event_type == EventType.AGENT_DIED
    assert world.gems_collected == 0


def test_world_gem_collected_and_agent_has_arrived():
    world = World(
        """
S0 X . .
.  . . .
G  . . ."""
    )
    # Do not collect the gem
    world.reset()

    # Collect the gem
    world.reset()
    world.step([Action.SOUTH])
    world.step([Action.SOUTH])
    assert world.gems_collected == 1

    # Exit the game
    world.step([Action.NORTH])
    world.step([Action.NORTH])
    world.step([Action.EAST])
    assert world.agents[0].has_arrived


def test_vertex_conflict():
    world = World(
        """
        .  X  .  .
        S0 .  S1  .
        .  X  .  ."""
    )
    world.reset()
    state = world.get_state()
    # Move to provoke a vertex conflict -> observations should remain identical
    world.step([Action.EAST, Action.WEST])
    new_state = world.get_state()
    assert state == new_state


def test_swapping_conflict():
    world = World(
        """
S0 X  .  .
.  .  S1  .
.  X  .  ."""
    )
    world.reset()
    world.step([Action.SOUTH, Action.WEST])
    try:
        world.step([Action.EAST, Action.WEST])
        raise Exception("These actions should not be allowed")
    except InvalidActionError:
        pass


def test_walk_into_laser_source():
    world = World(
        """
        @ L0S @ 
        .  .  . 
        X  .  S0
        .  .  ."""
    )
    world.reset()
    world.step([Action.WEST])
    world.step([Action.NORTH])
    with pytest.raises(ValueError):
        world.step([Action.NORTH])


def test_walk_outside_map():
    world = World(
        """@ @ L0S @  @
@ .  .  .  @
@ X  .  S0 @
@ .  .  .  @
@ @  .  @  @
"""
    )
    world.reset()
    world.step([Action.SOUTH])
    world.step([Action.WEST])
    world.step([Action.SOUTH])
    with pytest.raises(ValueError):
        world.step([Action.SOUTH])


def test_world_done():
    world = World(
        """
G  G  . .  S1
X  .  . @  .
@  .  G .  .
G  .  . G  X
@ L0N . S0 ."""
    )
    world.reset()
    world.step([Action.STAY, Action.WEST])
    world.step([Action.STAY, Action.WEST])
    world.step([Action.STAY, Action.WEST])
    try:
        world.step([Action.STAY, Action.WEST])
        raise Exception("The game should be finished")
    except ValueError:
        pass


def test_gems_collected():
    world = World("S0 G X")
    world.reset()
    assert world.gems_collected == 0
    world.step([Action.EAST])
    assert world.gems_collected == 1
    world.step([Action.EAST])
    assert world.gems_collected == 1


class StatusThread(Thread):
    INITIAL = 0
    FINISHED = 1

    def __init__(self, data):
        super().__init__()
        self.data = data
        self.status = StatusThread.INITIAL

    def run(self):
        print(self.data)
        self.status = StatusThread.FINISHED


def test_action_send_thread():
    t = StatusThread(Action.NORTH)
    t.start()
    t.join()
    assert t.status == StatusThread.FINISHED


def test_world_send_thread():
    world = World("S0 . X")
    t = StatusThread(world)
    t.start()
    t.join()
    assert t.status == StatusThread.FINISHED


def test_rendering_size():
    world = World("S0 . X")
    TILE_SIZE = 32
    expected_size = (TILE_SIZE * world.width + 1, TILE_SIZE * world.height + 1)
    assert world.image_dimensions == expected_size
    img = world.get_image()
    expected_shape = (expected_size[1], expected_size[0], 3)
    assert img.shape == expected_shape


def test_deepcopy():
    world = World("S0 . X")
    world2 = deepcopy(world)
    assert world.agents_positions == world2.agents_positions
    assert world.agents_positions is not world2.agents_positions
    assert world.width == world2.width


def test_deepcopy_not_initial_state():
    world = World("S0 . X")
    world.reset()
    world.step([Action.EAST])
    world2 = deepcopy(world)
    assert world.agents_positions == world2.agents_positions
    assert world.agents_positions is not world2.agents_positions
    assert world.width == world2.width


def test_get_state():
    world = World("S0 G X")
    world.reset()
    state = world.get_state()
    assert state.agents_positions == [(0, 0)]
    assert state.gems_collected == [False]
    world.step([Action.EAST])
    state = world.get_state()
    assert state.agents_positions == [(0, 1)]
    assert state.gems_collected == [True]


def test_set_state():
    world = World("S0 G X")
    world.reset()
    world.step([Action.EAST])
    events = world.set_state(WorldState([(0, 0)], [False]))
    assert world.agents_positions == [(0, 0)]
    assert world.gems_collected == 0
    assert len(events) == 0

    events = world.set_state(WorldState([(0, 2)], [True]))
    assert world.agents_positions == [(0, 2)]
    assert world.gems_collected == 1
    assert len(events) == 1
    assert events[0].agent_id == 0
    assert events[0].event_type == EventType.AGENT_EXIT


def test_set_invalid_state():
    world = World(
        """
        S1  S0 X
        L0E  G  X"""
    )
    world.reset()
    # Wrong number of gems
    with pytest.raises(InvalidWorldStateError):
        world.set_state(WorldState([(0, 0), (0, 1)], [True, True]))
    # Wrong number of agents
    with pytest.raises(InvalidWorldStateError):
        world.set_state(WorldState([(0, 0)], [True]))
    # Invalid agent position (out of bounds)
    with pytest.raises(InvalidWorldStateError):
        world.set_state(WorldState([(1, 1), (1, 0)], [True]))
    # Two agents on the same position
    with pytest.raises(InvalidWorldStateError):
        world.set_state(WorldState([(0, 0), (0, 0)], [True]))


def test_world_state_hash_eq():
    world = World("S0 G X")
    world.reset()
    state1 = world.get_state()
    state2 = world.get_state()
    assert hash(state1) == hash(state2)
    assert state1 == state2
    world.step([Action.EAST])
    state1 = world.get_state()
    state2 = world.get_state()
    assert hash(state1) == hash(state2)
    assert state1 == state2


def test_world_state_hash_neq():
    s1 = WorldState([(0, 0)], [False])
    s2 = WorldState([(0, 1)], [False])

    assert hash(s1) != hash(s2)
    assert s1 != s2


def test_get_standard_level():
    for i in range(1, 7):
        World.level(i)
        World.from_file(f"lvl{i}")
        World.from_file(f"level{i}")


def test_laser_tile_state():
    world = World("L0E S0 . X")
    world.reset()
    for _, laser in world.lasers:
        assert laser.is_off

    world.step([Action.EAST])
    for pos, laser in world.lasers:
        match pos:
            case (0, 1):
                assert laser.is_on
            case _:
                assert laser.is_off


def test_disable_deadly_laser_source_and_walk_into_it():
    world = World(
        """
        L0S . L0W X
        S0 S1  .  X
        """
    )
    world.reset()
    world.laser_sources[0, 2].disable()
    events = world.step([Action.STAY, Action.NORTH])
    assert len(events) == 0
    assert all(a.is_alive for a in world.agents)


def test_change_laser_colour():
    world = World(
        """
        L1E . S1 S0 X
        L0E .  .  . X
        """
    )
    world.reset()
    for (i, _), laser in world.lasers:
        if i == 0:
            assert laser.agent_id == 1
        else:
            assert laser.agent_id == 0

    bot_source = world.laser_sources[1, 0]

    NEW_COLOUR = 1
    bot_source.set_colour(NEW_COLOUR)
    world.reset()

    # Check that all the laser tiles have changed their colour
    for (i, _), laser in world.lasers:
        if i == 1:
            assert laser.agent_id == NEW_COLOUR
    events = world.step([Action.SOUTH, Action.SOUTH])
    assert len(events) == 0
    assert all(a.is_alive for a in world.agents)


def test_change_laser_colour_to_negative_colour():
    world = World("L0E S0 . X")
    world.reset()
    source = world.laser_sources[0, 0]

    try:
        source.set_colour(-1)
        raise Exception("Negative colours are not allowed")
    except ValueError:
        pass


def test_laser_colour_change_remains_after_reset():
    world = World("L0E S0 S1 X X")
    world.reset()
    source = world.laser_sources[0, 0]
    source.agent_id = 1
    world.reset()
    assert world.laser_sources[0, 0].agent_id == 1


def test_change_laser_colour_to_invalid_colour():
    world = World("L0E S0 . X")
    world.reset()
    source = world.laser_sources[0, 0]

    try:
        source.set_colour(2)
        raise Exception("This should not be allowed because there is only one agent in the world")
    except ValueError:
        pass

    try:
        source.set_colour(1)
        raise Exception("This should not be allowed because there is only one agent in the world")
    except ValueError:
        pass

    # Same test but by assigning the agent_id directly
    try:
        source.agent_id = 2
        raise Exception("This should not be allowed because there is only one agent in the world")
    except ValueError:
        pass

    try:
        source.agent_id = 1
        raise Exception("This should not be allowed because there is only one agent in the world")
    except ValueError:
        pass


def test_change_laser_colour_back():
    world = World(
        """
        L1E . S1 S0 X
        L0E .  .  . X
        """
    )
    world.reset()
    bot_source = world.laser_sources[1, 0]
    bot_source.set_colour(1)
    world.reset()
    assert world.laser_sources[1, 0].agent_id == 1
    for _, laser in world.lasers:
        assert laser.agent_id == 1

    bot_source.set_colour(0)
    world.reset()
    assert world.laser_sources[1, 0].agent_id == 0
    for (i, j), laser in world.lasers:
        if i == 0:
            assert laser.agent_id == 1
        elif i == 1:
            assert laser.agent_id == 0
