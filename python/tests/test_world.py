import random

import pytest

from laser_env.world import World, Action, REWARD_DEATH
from laser_env.tiles import AlternatingLaserSource, Laser


def test_available_actions():
    world = World("tests/maps/5x5_laser_2agents")
    world.reset()
    available_actions = world.available_actions()
    # Agent 0
    expected_available = [Action.NORTH, Action.WEST, Action.NOOP]
    assert all(expected in available_actions[0] for expected in expected_available)

    # Agent 1
    expected_available = [Action.WEST, Action.NOOP]
    assert all(expected in available_actions[1] for expected in expected_available)


def test_available_actions2():
    world = World("tests/maps/7x7_available_actions.txt")
    world.reset()

    def check(output: list[list[Action]], expected_agents_available_actions: list[list[Action]]):
        """
        For each agent, check that all the expected available actions are
        available and that all the expected not available actions are not available
        """
        for agent_output, agent_expected in zip(output, expected_agents_available_actions):
            for action in Action:
                if action in agent_expected:
                    assert action in agent_output
                else:
                    assert action not in agent_output

    check(world.available_actions(), [[Action.SOUTH, Action.WEST, Action.NOOP], [Action.SOUTH, Action.EAST, Action.NOOP]])
    # Move the agent to the end location and check the available actions
    for _ in range(3):
        world.step([Action.SOUTH, Action.SOUTH])
        check(
            world.available_actions(),
            [[Action.NORTH, Action.SOUTH, Action.WEST, Action.NOOP], [Action.NORTH, Action.SOUTH, Action.EAST, Action.NOOP]],
        )
    world.step([Action.SOUTH, Action.SOUTH])
    check(world.available_actions(), [[Action.NOOP], [Action.NOOP]])


def test_parse_wrong_worlds():
    # Not enough finish tiles for all the agents
    with pytest.raises(AssertionError):
        World("tests/maps/wrong.txt")

    # Zero agent in the environment
    with pytest.raises(AssertionError):
        World("tests/maps/3x3_0agent")


def test_world_move():
    world = World("tests/maps/3x4.txt")
    world.reset()
    world.step([Action.SOUTH])
    world.step([Action.EAST])
    world.step([Action.NORTH])


def test_end_game_void():
    world = World("tests/maps/5x5void_two_agents.txt")
    world.reset()
    # Kill no one
    _, stop = world.step([Action.NOOP, Action.NOOP])
    assert not stop, "The game should not be finished"

    # Kill one agent only
    world.reset()
    _, stop = world.step([Action.EAST, Action.NOOP])
    assert stop, "The game should be finished because one agent has died"

    world.reset()
    _, stop = world.step([Action.NOOP, Action.EAST])
    assert stop, "The game should be finished because one agent has died"

    # Kill both agents
    world.reset()
    _, stop = world.step([Action.EAST, Action.EAST])
    assert stop, "The game should be finished because all agents have died"


def test_time_reward():
    world = World("tests/maps/basic.txt")
    world.reset()
    for action in Action:
        reward, _ = world.step([action])
        assert reward == 0


def test_finish_reward():
    from laser_env.world import REWARD_SUCCESS, REWARD_ARRIVED

    world = World("tests/maps/basic.txt")
    world.reset()
    world.step([Action.EAST])
    reward, _ = world.step([Action.SOUTH])
    assert reward == REWARD_SUCCESS + REWARD_ARRIVED


def test_collect_reward():
    from laser_env.world import REWARD_COLLECT

    world = World("tests/maps/3x4_gem.txt")
    world.reset()
    world.step([Action.SOUTH])
    reward, _ = world.step([Action.SOUTH])
    assert reward == REWARD_COLLECT


def test_walk_into_wall():
    world = World("tests/maps/basic.txt")
    world.reset()
    world.step([Action.SOUTH])
    with pytest.raises(AssertionError) as _:
        world.step([Action.SOUTH])


def test_long_play():
    world = World("tests/maps/basic.txt")
    world.reset()
    for _ in range(10_000):
        available = [[a for a in agent_actions] for agent_actions in world.available_actions()]
        action = [random.choice(a) for a in available]
        _, stop = world.step(action)
        if stop:
            world.reset()


def test_world_success():
    world = World("tests/maps/3x4_gem.txt")
    # Do not collect the gem
    world.reset()
    _, done = world.step([Action.EAST])
    assert done

    # Collect the gem
    world.reset()
    world.step([Action.SOUTH])
    world.step([Action.SOUTH])
    world.step([Action.NORTH])
    world.step([Action.NORTH])
    _, done = world.step([Action.EAST])
    assert done


def test_replay():
    world = World("tests/maps/3x4_gem.txt")
    from laser_env.world import REWARD_COLLECT, REWARD_SUCCESS, REWARD_ARRIVED

    def play():
        """Collect the gem and finish the game. Check that the reward is is correct when collecting it."""
        world.reset()
        world.step([Action.SOUTH])
        reward, stop = world.step([Action.SOUTH])
        assert reward == REWARD_COLLECT
        assert not stop
        r, _ = world.step([Action.NORTH])
        assert r == 0
        r, _ = world.step([Action.NORTH])
        assert r == 0
        reward, stop = world.step([Action.EAST])
        assert stop == True
        assert reward == REWARD_SUCCESS + REWARD_ARRIVED

    for _ in range(10):
        play()


def test_vertex_conflict():
    world = World("tests/maps/3x4_two_agents.txt")
    world.reset()
    world.step([Action.SOUTH, Action.WEST])
    # Move to provoke a vertex conflict -> observations should remain identical
    with pytest.raises(AssertionError) as _:
        world.step([Action.NOOP, Action.WEST])


def test_swapping_conflict():
    world = World("tests/maps/3x4_two_agents.txt")
    world.reset()
    world.step([Action.SOUTH, Action.WEST])
    # Move to provoke a swapping conflict -> observations should remain identical
    with pytest.raises(AssertionError) as _:
        world.step([Action.EAST, Action.WEST])


def test_walk_into_laser_source():
    world = World("tests/maps/5x5_laser_1agent")
    world.reset()
    world.step([Action.WEST])
    world.step([Action.NORTH])
    with pytest.raises(AssertionError) as _:
        world.step([Action.NORTH])


def test_walk_outside_map():
    world = World("tests/maps/5x5_laser_no_wall")
    world.reset()
    _, stop = world.step([Action.SOUTH])
    assert not stop
    _, stop = world.step([Action.WEST])
    assert not stop
    _, stop = world.step([Action.SOUTH])
    assert not stop
    with pytest.raises(AssertionError) as _:
        world.step([Action.SOUTH])


def test_world_done():
    world = World("tests/maps/one_laser")
    world.reset()
    world.step([Action.NOOP, Action.WEST])
    world.step([Action.NOOP, Action.WEST])
    world.step([Action.NOOP, Action.WEST])
    try:
        world.step([Action.NOOP, Action.WEST])
        raise Exception("The game should be finished")
    except AssertionError:
        pass


def test_indexing_out_of_map():
    world = World("tests/maps/one_laser")
    world.reset()
    try:
        world[50, 50]
        assert False
    except IndexError:
        pass


def test_negative_indexing():
    world = World("tests/maps/one_laser")
    world.reset()
    try:
        world[-1, -1]
        assert False
    except IndexError:
        pass


def test_force_state():
    world = World.from_str(
        """
    . . .
    S0 . .
    . . F
    """
    )
    world.reset()

    world.force_state([(0, 0)], [False])
    assert world[0, 0].is_occupied
    assert world[1, 0].is_empty


def test_force_state_with_gem():
    world = World.from_str(
        """
    . . .
    S0 . G
    . . F
    """
    )
    world.reset()

    world.force_state([(0, 0)], [False])
    assert world[0, 0].is_occupied
    assert world[1, 0].is_empty

    world.force_state([(0, 2)], [True])
    assert world[0, 2].is_occupied
    assert world[0, 0].is_empty
    assert all(gem.collected for gem in world._gems)


def test_force_state_agent_dead():
    world = World.from_str(
        """
    L1E . .
     S0 . G
      . . F
    """
    )
    world.reset()
    assert not world.done
    world.force_state([(0, 1)], [False])
    assert world.done


def test_force_end_state():
    world = World.from_str(
        """
    L1E . .
     S0 . G
      . . F
    """
    )
    world.reset()
    assert not world.done
    # Without collecting the gem
    world.force_state([(2, 2)], [False])
    assert world.done
    # With collecting the gem
    world.force_state([(2, 2)], [True])
    assert world.done


def test_reward_kills_on_end():
    world = World.from_str("G @ . . G\n. F G L0S S0\n. . G . .\n. @ L1E F @\n. . S1 G .")
    world.reset()
    world.force_state([(2, 3), (1, 1)], [False] * 5)
    reward, done = world.step([Action.SOUTH, Action.NOOP])
    assert done
    assert reward == REWARD_DEATH


def test_parse_changing_laser():
    world = World.from_str(
        """
    ~L1E . .
     S0  . G
      .  . F
    """
    )
    world.reset()
    laser_source = world[0, 0]
    l1 = world[0, 1]
    l2 = world[0, 2]
    assert isinstance(laser_source, AlternatingLaserSource)
    assert isinstance(l1, Laser)
    assert isinstance(l2, Laser)


def test_reset_alternating_laser():
    world = World.from_str(
        """
    ~L2E . .
     S0  . G
      .  . F
    """
    )
    world.reset()
    laser_source = world[0, 0]
    assert isinstance(laser_source, AlternatingLaserSource)
    first_colour = laser_source.agent_id
    l1 = world[0, 1]
    l2 = world[0, 2]
    assert isinstance(l1, Laser)
    assert isinstance(l2, Laser)
    once_different = False
    for i in range(100):
        world.reset()
        colour = laser_source.agent_id
        assert l1.agent_id == colour
        assert l2.agent_id == colour
        if colour != first_colour:
            once_different = True
    assert once_different, "The laser colour should change at least once"


def test_alternating_laser_with_id_0():
    try:
        world = World.from_str(
            """
        ~L0E . .
        S0  . G
        .  . F
        """
        )
    except AssertionError:
        return
    assert False, "The alternating laser source should not be created"


def test_world_get_as_str():
    world = World.from_str(
        """
        ~L1E . @
        S0  . G
        V  . F
        """
    )
    world.reset()
    assert world.to_world_string(static=False) == "~L1E . @\nS0 . G\nV . F"
    assert world.to_world_string(static=True) == "L0E . @\nS0 . G\nV . F"
