from typing import Any
from lle import LLE, Action, REWARD_AGENT_DIED, REWARD_AGENT_EXIT, REWARD_END_GAME, REWARD_GEM_COLLECTED, WorldState
from copy import deepcopy
import numpy as np


def test_available_actions():
    env = LLE.from_str(
        """
@ @ L0S @  @
@ .  .  .  @
@ X  .  S0 @
@ X  .  S1 @
@ @  @  @  @
"""
    )
    env.reset()
    available_actions = env.available_actions()
    # Agent 0
    assert available_actions[0, Action.NORTH.value] == 1
    assert available_actions[0, Action.EAST.value] == 0
    assert available_actions[0, Action.SOUTH.value] == 0
    assert available_actions[0, Action.WEST.value] == 1
    assert available_actions[0, Action.STAY.value] == 1

    # Agent 1
    assert available_actions[1, Action.NORTH.value] == 0
    assert available_actions[1, Action.EAST.value] == 0
    assert available_actions[1, Action.SOUTH.value] == 0
    assert available_actions[1, Action.WEST.value] == 1
    assert available_actions[1, Action.STAY.value] == 1


def test_available_actions2():
    env = LLE.from_str(
        """
@   @ @  @  @ @ @
@   . S0 S1 . . @
@   . .  .  . . @
L0E . .  .  . . @
@   . .  .  . G @
@   . X  X  . . @
@   @ @  @  @ @ @"""
    )
    obs = env.reset()

    def check_available_actions(available: np.ndarray[np.int32, Any], expected_available: list[list[Action]]) -> bool:
        available_actions = np.zeros((2, Action.N), dtype=np.int32)
        for agent_id, actions in enumerate(expected_available):
            for action in actions:
                available_actions[agent_id, action.value] = 1
        return np.array_equal(available, available_actions)

    assert check_available_actions(
        obs.available_actions, [[Action.SOUTH, Action.WEST, Action.STAY], [Action.SOUTH, Action.EAST, Action.STAY]]
    )

    # Move the agent to the end location and check the available actions
    for _ in range(3):
        obs, *_ = env.step([Action.SOUTH.value, Action.SOUTH.value])
        check_available_actions(
            obs.available_actions,
            [[Action.NORTH, Action.SOUTH, Action.WEST, Action.STAY], [Action.NORTH, Action.SOUTH, Action.EAST, Action.STAY]],
        )
    obs, *_ = env.step([Action.SOUTH.value, Action.SOUTH.value])
    check_available_actions(obs.available_actions, [[Action.STAY] * 2])


def test_width_height():
    env = LLE.from_str(
        """S0 X .
.  . .
.  . ."""
    )
    assert env.width == 3
    assert env.height == 3

    env = LLE.from_str(
        """S0 X . .
.  . . .
G  . . ."""
    )
    assert env.width == 4
    assert env.height == 3


def test_state():
    env = LLE.from_str(
        """S0 X .
.  . .
.  . ."""
    )
    assert env.state_shape == (env.n_agents * 2 + env.world.n_gems,)
    env.reset()
    state = env.get_state()
    assert state.shape == env.state_shape


def test_action_meanings():
    env = LLE.from_str(
        """S0 X .
.  . .
.  . ."""
    )
    assert env.action_space.action_names == [a.name for a in Action.ALL]


def test_deep_copy():
    env = LLE.from_str("S0 X")
    copy = deepcopy(env)
    assert env is not copy

    env.reset()
    copy.reset()

    _, _, done, _, _ = env.step([Action.EAST.value])
    assert done
    # If the deepcopy is not correct, the copy should also be done and the game should crash
    # If the deepcopy is properly done, then the copy should not be done
    _, _, done, _, _ = copy.step([Action.STAY.value])
    assert not done


def test_move_end_game():
    env = LLE.from_str(
        """
    S0 X .
    .  . .
    .  . .""",
    )
    env.reset()
    env.step([Action.SOUTH.value])
    assert not env.done
    env.step([Action.SOUTH.value])
    assert not env.done
    env.step([Action.EAST.value])
    assert not env.done
    env.step([Action.NORTH.value])
    assert not env.done
    env.step([Action.NORTH.value])
    assert env.done


def test_time_reward():
    env = LLE.from_str(
        """
    . .  . X
    . S0 . .
    . .  . ."""
    )
    env.reset()
    for action in Action.ALL:
        _obs, reward, *_ = env.step([action.value])
        assert reward == 0


def test_finish_reward():
    world = LLE.from_str(
        """@ @ @  @ @ @
@ . .  . . @
@ . S0 . . @
@ . .  X . @
@ @ @  @ @ @"""
    )
    world.reset()
    world.step([Action.EAST.value])
    reward = world.step([Action.SOUTH.value])[1]
    assert reward == REWARD_END_GAME + REWARD_AGENT_EXIT


def test_arrive_reward_only_once():
    """Some kind of adversarial game where only one agent can move at a time."""
    env = LLE.from_str(
        """
    S0 . G
    S1 X X
"""
    )
    action_rewards = [
        ([Action.EAST, Action.STAY], 0),  # Agent 0
        ([Action.STAY, Action.EAST], 1),  # Agent 1 finishes the game
        ([Action.STAY, Action.STAY], 0),  # Agent 0
        ([Action.STAY, Action.STAY], 0),  # Agent 1
        ([Action.EAST, Action.STAY], 1),  # Agent 0 collects the gem
        ([Action.STAY, Action.STAY], 0),  # Agent 1
        ([Action.SOUTH, Action.STAY], 2),  # Agent 0 finishes the game
    ]
    env.reset()
    for action, reward in action_rewards:
        assert env.step([a.value for a in action])[1] == reward


def test_void_reward():
    env = LLE.from_str("S0 V X")
    env.reset()
    assert env.step([Action.EAST.value])[1] == REWARD_AGENT_DIED
    assert env.done


def test_collect_reward():
    env = LLE.from_str(
        """S0 X . .
.  . . .
G  . . ."""
    )
    env.reset()
    env.step([Action.SOUTH.value])
    reward = env.step([Action.SOUTH.value])[1]
    assert reward == REWARD_GEM_COLLECTED


def test_reward_after_reset():
    env = LLE.from_str(
        """
    S0 X . .
    .  . . .
    G  . . .
    """
    )

    def play():
        """Collect the gem and finish the game. Check that the reward is is correct when collecting it."""
        env.reset()
        env.step([Action.SOUTH.value])
        reward = env.step([Action.SOUTH.value]).reward
        assert reward == REWARD_GEM_COLLECTED
        assert not env.done
        r = env.step([Action.NORTH.value]).reward
        assert r == 0
        r = env.step([Action.NORTH.value]).reward
        assert r == 0
        reward = env.step([Action.EAST.value]).reward
        assert env.done
        assert reward == REWARD_END_GAME + REWARD_AGENT_EXIT

    for _ in range(10):
        play()


def test_reward_after_set_state():
    """After forcing the state of the environment, make sure that
    the reward is correct, and does not include the reward of
    forcing the state."""
    env = LLE.from_str(
        """
    S0 . G
    S1 X X""",
    )
    env.reset()
    state = WorldState([(0, 1), (1, 1)], [False])
    env.set_state(state)
    assert env.step([Action.EAST.value, Action.STAY.value]).reward == 1.0


def test_reward_set_state_all_arrived():
    env = LLE.from_str(
        """
    S0 . G
    S1 X X""",
    )
    env.reset()
    state = WorldState([(0, 2), (1, 1)], [True])
    env.set_state(state)
    assert env.step([Action.SOUTH.value, Action.STAY.value]) != REWARD_END_GAME + REWARD_AGENT_EXIT


def test_set_state():
    env = LLE.from_str("S0 G X")
    env.reset()
    env.step([Action.EAST.value])
    env.set_state(WorldState([(0, 0)], [False]))
    assert env.world.agents_positions == [(0, 0)]
    assert env.world.gems_collected == 0
    assert not env.done

    env.set_state(WorldState([(0, 2)], [True]))
    assert env.world.agents_positions == [(0, 2)]
    assert env.world.gems_collected == 1
    assert env.done


# Test cases
def test_reward():
    env = LLE.from_str(
        """
    S0 G .
    .  . X
    """
    )
    env.reset()
    assert env.step([Action.EAST.value]).reward == REWARD_GEM_COLLECTED
    assert env.step([Action.EAST.value]).reward == 0.0
    assert env.step([Action.SOUTH.value]).reward == REWARD_AGENT_EXIT + REWARD_END_GAME


def test_reward_death():
    env = LLE.from_str(
        """
    S0 L0S X
    S1  .  X
    """
    )
    env.reset()
    assert env.step([Action.STAY.value, Action.EAST.value]).reward == REWARD_AGENT_DIED
    assert env.done


def test_reward_collect_and_death():
    env = LLE.from_str(
        """
    S0 L0S X
    S1  G  X
    """
    )
    env.reset()
    assert env.step([Action.STAY.value, Action.EAST.value]).reward == REWARD_AGENT_DIED
    assert env.done


def test_force_end_state():
    env = LLE.from_str(
        """
        S0 . G
        X  . .
    """,
    )
    env.reset()
    s = WorldState([(1, 0)], [True])
    env.set_state(s)
    assert env.done


def test_force_state_agent_dies():
    env = LLE.from_str(
        """
        S0 S1 G
        X  X L0W
    """,
    )
    env.reset()

    s = WorldState([(1, 0), (1, 1)], [False])
    env.set_state(s)
    assert env.done
