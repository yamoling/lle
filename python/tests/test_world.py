import random

import pytest
from copy import deepcopy

from lle import World, Action, REWARD_END_GAME, REWARD_AGENT_JUST_ARRIVED, REWARD_GEM_COLLECTED


def test_available_actions():
    world = World.from_file("python/tests/maps/5x5_laser_2agents")
    world.reset()
    available_actions = world.available_actions()
    # available_actions = [[a.value for a in available] for available in available_actions]
    # Agent 0
    expected = [[Action.NORTH, Action.WEST, Action.STAY], [Action.WEST, Action.STAY]]
    for e, available in zip(expected, available_actions):
        for a in available:
            assert a in e, f"Action {a} not in {e}"


def test_parse_wrong_worlds():
    # Not enough finish tiles for all the agents
    with pytest.raises(ValueError):
        World(
            """
            @ @  @ @
            @ S0 . @
            @ .  . @
            @ @  @ @"""
        )

    # Zero agent in the environment
    with pytest.raises(ValueError):
        World("X G")


def test_world_move():
    world = World.from_file("python/tests/maps/3x4.txt")
    world.reset()
    world.step([Action.SOUTH])
    world.step([Action.EAST])
    world.step([Action.NORTH])


def test_time_reward():
    world = World(
        """
    . .  . X
    . S0 . .
    . .  . ."""
    )
    world.reset()
    for action in Action.ALL:
        reward = world.step([action])
        assert reward == 0


def test_finish_reward():
    world = World.from_file("python/tests/maps/basic.txt")
    world.reset()
    world.step([Action.EAST])
    reward = world.step([Action.SOUTH])
    assert reward == REWARD_END_GAME + REWARD_AGENT_JUST_ARRIVED


def test_collect_reward():
    world = World.from_file("python/tests/maps/3x4_gem.txt")
    world.reset()
    world.step([Action.SOUTH])
    reward = world.step([Action.SOUTH])
    assert reward == REWARD_GEM_COLLECTED


def test_walk_into_wall():
    world = World.from_file("python/tests/maps/basic.txt")
    world.reset()
    world.step([Action.SOUTH])
    with pytest.raises(ValueError):
        world.step([Action.SOUTH])


def test_long_play():
    world = World.from_file("python/tests/maps/basic.txt")
    world.reset()
    for _ in range(10_000):
        available = [[a for a in agent_actions] for agent_actions in world.available_actions()]
        action = [random.choice(a) for a in available]
        world.step(action)
        if world.done:
            world.reset()


def test_world_success():
    world = World.from_file("python/tests/maps/3x4_gem.txt")
    # Do not collect the gem
    world.reset()
    world.step([Action.EAST])
    assert world.done

    # Collect the gem
    world.reset()
    world.step([Action.SOUTH])
    world.step([Action.SOUTH])
    world.step([Action.NORTH])
    world.step([Action.NORTH])
    world.step([Action.EAST])
    assert world.done


def test_replay():
    world = World.from_file("python/tests/maps/3x4_gem.txt")

    def play():
        """Collect the gem and finish the game. Check that the reward is is correct when collecting it."""
        world.reset()
        world.step([Action.SOUTH])
        reward = world.step([Action.SOUTH])
        assert reward == REWARD_GEM_COLLECTED
        assert not world.done
        r = world.step([Action.NORTH])
        assert r == 0
        r = world.step([Action.NORTH])
        assert r == 0
        reward = world.step([Action.EAST])
        assert world.done
        assert reward == REWARD_END_GAME + REWARD_AGENT_JUST_ARRIVED

    for _ in range(10):
        play()


def test_vertex_conflict():
    world = World.from_file("python/tests/maps/3x4_two_agents.txt")
    world.reset()
    world.step([Action.SOUTH, Action.WEST])
    # Move to provoke a vertex conflict -> observations should remain identical
    with pytest.raises(ValueError) as _:
        world.step([Action.STAY, Action.WEST])


def test_swapping_conflict():
    world = World.from_file("python/tests/maps/3x4_two_agents.txt")
    world.reset()
    world.step([Action.SOUTH, Action.WEST])
    # Move to provoke a swapping conflict -> observations should remain identical
    with pytest.raises(ValueError) as _:
        world.step([Action.EAST, Action.WEST])


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
    world = World.from_file("python/tests/maps/5x5_laser_no_wall")
    world.reset()
    world.step([Action.SOUTH])
    assert not world.done
    world.step([Action.WEST])
    assert not world.done
    world.step([Action.SOUTH])
    assert not world.done
    with pytest.raises(ValueError):
        world.step([Action.SOUTH])


def test_world_done():
    world = World.from_file("python/tests/maps/one_laser")
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


from threading import Thread


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
    assert world.agent_positions == world2.agent_positions
    assert world.agent_positions is not world2.agent_positions
    assert world.width == world2.width


def test_deepcopy_not_initial_state():
    world = World("S0 . X")
    world.reset()
    world.step([Action.EAST])
    world2 = deepcopy(world)
    assert world.agent_positions == world2.agent_positions
    assert world.agent_positions is not world2.agent_positions
    assert world.width == world2.width


def test_get_state():
    world = World("S0 G X")
    world.reset()
    state = world.get_state()
    assert state == ([(0, 0)], [False])
    world.step([Action.EAST])
    state = world.get_state()
    assert state == ([(0, 1)], [True])


def test_set_state():
    world = World("S0 G X")
    world.reset()
    world.step([Action.EAST])
    world.set_state([(0, 0)], [False])
    assert world.agent_positions == [(0, 0)]
    assert world.gems_collected == 0
    assert not world.done

    world.set_state([(0, 2)], [True])
    assert world.agent_positions == [(0, 2)]
    assert world.gems_collected == 1
    assert world.done
