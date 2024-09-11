from lle import LLE, Action, WorldState, ObservationType
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
    ).core()
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
    ).core()
    obs = env.reset()

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
    ).core()
    assert env.width == 3
    assert env.height == 3

    env = LLE.from_str(
        """S0 X . .
.  . . .
G  . . ."""
    ).core()
    assert env.width == 4
    assert env.height == 3


def test_state_default():
    env = LLE.from_str(
        """S0 X .
                        .  . .
                        .  . ."""
    ).core()
    assert env.state_shape == (env.n_agents * 2 + env.world.n_gems,)
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
        .core()
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
    ).core()
    assert env.action_space.action_names == [a.name for a in Action.ALL]


def test_deep_copy():
    env = LLE.from_str("S0 X").core()
    copy = deepcopy(env)
    assert env is not copy

    env.reset()
    copy.reset()

    _, done, _, _ = env.step([Action.EAST.value])
    assert done
    # If the deepcopy is not correct, the copy should also be done and the game should crash
    # If the deepcopy is properly done, then the copy should not be done
    _, done, _, _ = copy.step([Action.STAY.value])
    assert not done


def test_move_end_game():
    env = LLE.from_str(
        """
    S0 X .
    .  . .
    .  . .""",
    ).core()
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


def test_force_end_state():
    env = LLE.from_str(
        """
        S0 . G
        X  . .
    """,
    ).single_objective()
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
    ).core()
    env.reset()

    s = WorldState([(1, 0), (1, 1)], [False])
    env.set_state(s)
    assert env.done


def test_agent_state_size():
    env = LLE.level(1).core()
    assert env.agent_state_size == 2

    env = LLE.level(1).state_type(ObservationType.FLATTENED).core()
    try:
        env.agent_state_size
        assert False, "So far, only state generators of type `StateGenerator` have a `agent_state_size`."
    except (ValueError, NotImplementedError):
        pass


def test_builder_obs_type_string():
    env = LLE.level(1).obs_type("layered").single_objective()
    env2 = LLE.level(1).obs_type(ObservationType.LAYERED).single_objective()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("flattened").single_objective()
    env2 = LLE.level(1).obs_type(ObservationType.FLATTENED).single_objective()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("partial3x3").single_objective()
    env2 = LLE.level(1).obs_type(ObservationType.PARTIAL_3x3).single_objective()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("partial5x5").single_objective()
    env2 = LLE.level(1).obs_type(ObservationType.PARTIAL_5x5).single_objective()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("partial7x7").single_objective()
    env2 = LLE.level(1).obs_type(ObservationType.PARTIAL_7x7).single_objective()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("state").single_objective()
    env2 = LLE.level(1).obs_type(ObservationType.STATE).single_objective()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("image").single_objective()
    env2 = LLE.level(1).obs_type(ObservationType.RGB_IMAGE).single_objective()
    assert env.has_same_inouts(env2)

    env = LLE.level(1).obs_type("perspective").single_objective()
    env2 = LLE.level(1).obs_type(ObservationType.AGENT0_PERSPECTIVE_LAYERED).single_objective()
    assert env.has_same_inouts(env2)
