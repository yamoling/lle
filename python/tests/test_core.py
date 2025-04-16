from lle import LLE, Action, WorldState, ObservationType
from copy import deepcopy
import numpy as np
import pytest


def test_available_actions():
    env = LLE.from_str(
        """
@ @ L0S @  @
@ .  .  .  @
@ X  .  S0 @
@ X  .  S1 @
@ @  @  @  @
"""
    ).build()
    env.reset()
    available_actions = env.available_actions()
    # Agent 0
    assert available_actions[0, Action.NORTH.value]
    assert not available_actions[0, Action.EAST.value]
    assert not available_actions[0, Action.SOUTH.value]
    assert available_actions[0, Action.WEST.value]
    assert available_actions[0, Action.STAY.value]

    # Agent 1
    assert not available_actions[1, Action.NORTH.value]
    assert not available_actions[1, Action.EAST.value]
    assert not available_actions[1, Action.SOUTH.value]
    assert available_actions[1, Action.WEST.value]
    assert available_actions[1, Action.STAY.value]


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
    ).build()
    obs, state = env.reset()

    def check_available_actions(available: np.ndarray, expected_available: list[list[Action]]) -> bool:
        available_actions = np.full((2, Action.N), False, dtype=bool)
        for agent_id, actions in enumerate(expected_available):
            for action in actions:
                available_actions[agent_id, action.value] = True
        return np.array_equal(available, available_actions)

    assert check_available_actions(
        obs.available_actions, [[Action.SOUTH, Action.WEST, Action.STAY], [Action.SOUTH, Action.EAST, Action.STAY]]
    )

    # Move the agent to the end location and check the available actions
    for _ in range(3):
        step = env.step([Action.SOUTH.value, Action.SOUTH.value])
        check_available_actions(
            step.obs.available_actions,
            [[Action.NORTH, Action.SOUTH, Action.WEST, Action.STAY], [Action.NORTH, Action.SOUTH, Action.EAST, Action.STAY]],
        )
    step = env.step([Action.SOUTH.value, Action.SOUTH.value])
    check_available_actions(step.obs.available_actions, [[Action.STAY] * 2])


def test_width_height():
    env = LLE.from_str(
        """S0 X .
.  . .
.  . ."""
    ).build()
    assert env.width == 3
    assert env.height == 3

    env = LLE.from_str(
        """S0 X . .
.  . . .
G  . . ."""
    ).build()
    assert env.width == 4
    assert env.height == 3


def test_state_default():
    env = LLE.from_str(
        """S0 X .
                        .  . .
                        .  . ."""
    ).build()
    assert env.state_shape == (env.n_agents * 3 + env.world.n_gems,)
    env.reset()
    state = env.get_state()
    assert state.shape == env.state_shape


def test_state_flattened():
    env = (
        LLE.from_str(
            """S0 X .
.  . .
.  . .""",
        )
        .state_type(ObservationType.FLATTENED)
        .build()
    )
    assert env.state_shape == (np.prod(env.state_shape),)
    env.reset()
    state = env.get_state()
    assert state.shape == env.state_shape


def test_action_meanings():
    env = LLE.from_str(
        """S0 X .
.  . .
.  . ."""
    ).build()
    for individual_space in env.action_space.spaces:
        assert individual_space.labels == [a.name for a in Action.ALL]


def test_deep_copy():
    env = LLE.from_str("S0 X").build()
    copy = deepcopy(env)
    assert env is not copy

    env.reset()
    copy.reset()

    done = env.step([Action.EAST.value]).done
    assert done
    # If the deepcopy is not correct, the copy should also be done and the game should crash
    # If the deepcopy is properly done, then the copy should not be done
    done = copy.step([Action.STAY.value]).done
    assert not done


def test_move_end_game():
    env = LLE.from_str(
        """
    S0 X .
    .  . .
    .  . .""",
    ).build()
    env.reset()
    done = env.step([Action.SOUTH.value]).done
    assert not done
    done = env.step([Action.SOUTH.value]).done
    assert not done
    done = env.step([Action.EAST.value]).done
    assert not done
    done = env.step([Action.NORTH.value]).done
    assert not done
    done = env.step([Action.NORTH.value]).done
    assert done


def test_force_end_state():
    env = LLE.from_str(
        """
        S0 . G
        X  . .
    """,
    ).build()
    env.reset()
    s = WorldState([(1, 0)], [True])
    env.set_state(s)
    assert env.done


def test_force_state_agent_dies():
    env = LLE.from_str(
        """
        S0 S1 G
        X  . L0W
        .  X  .
    """,
    ).build()
    env.reset()

    ws = WorldState([(1, 0), (1, 1)], [False], [True, False])
    env.set_state(ws)

    with pytest.raises(ValueError):
        available = env.available_actions()
        env.step(env.action_space.sample(available))


def test_agent_state_size():
    env = LLE.level(1).build()
    assert env.agent_state_size == 2

    env = LLE.level(1).state_type(ObservationType.FLATTENED).build()
    with pytest.raises((ValueError, NotImplementedError)):
        env.agent_state_size


def test_builder_obs_type_string():
    env = LLE.level(1).obs_type("layered").build()
    env2 = LLE.level(1).obs_type(ObservationType.LAYERED).build()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("flattened").build()
    env2 = LLE.level(1).obs_type(ObservationType.FLATTENED).build()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("partial3x3").build()
    env2 = LLE.level(1).obs_type(ObservationType.PARTIAL_3x3).build()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("partial5x5").build()
    env2 = LLE.level(1).obs_type(ObservationType.PARTIAL_5x5).build()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("partial7x7").build()
    env2 = LLE.level(1).obs_type(ObservationType.PARTIAL_7x7).build()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("state").build()
    env2 = LLE.level(1).obs_type(ObservationType.STATE).build()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("image").build()
    env2 = LLE.level(1).obs_type(ObservationType.RGB_IMAGE).build()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("perspective").build()
    env2 = LLE.level(1).obs_type(ObservationType.AGENT0_PERSPECTIVE_LAYERED).build()
    assert env.has_same_inouts(env2)


def test_n_agents():
    env = LLE.from_str("S0 S1 X X").build()
    env.reset()
    assert env.n_agents == 2

    import marlenv

    env = LLE.level(6).obs_type(ObservationType.LAYERED).state_type(ObservationType.STATE).build()
    env = marlenv.Builder(env).agent_id().time_limit(78, add_extra=True).build()
    assert env.n_agents == 4
    env.reset()
    assert env.n_agents == 4
