from typing import Any
from lle import LLE, Action
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
    assert env.state_shape == (1 * 5 * 3 * 3,)
    env.reset()
    state = env.get_state()
    assert state.shape == env.state_shape


def test_action_meanings():
    env = LLE.from_str(
        """S0 X .
.  . .
.  . ."""
    )
    assert env.action_meanings == [a.name for a in Action.ALL]


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
