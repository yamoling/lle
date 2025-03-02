import tempfile
import os
from lle import LLE, Action, WorldState
from lle.env.reward_strategy import REWARD_DEATH, REWARD_GEM, REWARD_EXIT, REWARD_DONE, MultiObjective
import numpy as np


def test_void_reward():
    env = LLE.from_str("S0 V X").build()
    env.reset()
    _, _, reward, done, _, _ = env.step([Action.EAST.value])
    assert reward == REWARD_DEATH
    assert done


def test_collect_reward():
    env = LLE.from_str(
        """S0 X . .
.  . . .
G  . . ."""
    ).build()
    env.reset()
    env.step([Action.SOUTH.value])
    reward = env.step([Action.SOUTH.value]).reward
    assert reward == REWARD_GEM


def test_time_reward():
    env = LLE.from_str(
        """
    . .  . X
    . S0 . .
    . .  . ."""
    ).build()
    env.reset()
    for action in Action.ALL:
        reward = env.step([action.value]).reward
        assert reward == 0


def test_finish_reward():
    env = LLE.from_str(
        """@ @ @  @ @ @
@ . .  . . @
@ . S0 . . @
@ . .  X . @
@ @ @  @ @ @"""
    ).build()
    env.reset()
    env.step([Action.EAST.value])
    reward = env.step([Action.SOUTH.value]).reward
    assert reward == REWARD_DONE + REWARD_EXIT


def test_arrive_reward_only_once():
    """Some kind of adversarial game where only one agent can move at a time."""
    env = LLE.from_str(
        """
    S0 . G
    S1 X X
""",
    ).build()
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
        r = env.step([a.value for a in action]).reward
        assert r == reward


def test_reward_after_reset():
    env = LLE.from_str(
        """
    S0 X . .
    .  . . .
    G  . . .
    """
    ).build()

    def play():
        """Collect the gem and finish the game. Check that the reward is is correct when collecting it."""
        env.reset()
        env.step([Action.SOUTH.value])
        reward = env.step([Action.SOUTH.value]).reward
        assert reward == REWARD_GEM
        assert not env.done
        r = env.step([Action.NORTH.value]).reward
        assert r == 0
        r = env.step([Action.NORTH.value]).reward
        assert r == 0
        reward = env.step([Action.EAST.value]).reward
        assert env.done
        assert reward == REWARD_DONE + REWARD_EXIT

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
    ).build()
    env.reset()
    world_state = WorldState([(0, 1), (1, 1)], [False])
    env.set_state(world_state)
    assert env.step([Action.EAST.value, Action.STAY.value]).reward == REWARD_GEM


def test_reward_set_state_all_arrived():
    env = (
        LLE.from_str(
            """
    S0 . G
    S1 X X""",
        )
        .state_type("state")
        .build()
    )

    world_state = WorldState([(0, 2), (1, 1)], [True])
    env.world.set_state(world_state)
    state = env.get_state()
    env.reset()

    env.set_state(state)
    r = env.step([Action.SOUTH.value, Action.STAY.value]).reward
    assert r == REWARD_DONE + REWARD_EXIT


def test_set_state():
    # Walkable lasers must be set to false, otherwise agents could die
    env = (
        LLE.from_str("""
width=10
height=10
exits = [{ j_min = 9 }]
[[agents]]
start_positions = [{ }] # random spawn

[[agents]]
start_positions = [{ }]        

[[agents]]
start_positions = [{ }]

[[agents]]
start_positions = [{ }]""")
        .state_type("state")
        .build()
    )
    states = [env.reset()[1]]
    world_states = [env.world.get_state()]
    i = 0
    done = False
    while not done and i < 1000:
        i += 1
        step = env.step(env.action_space.sample(env.available_actions()))
        world_states.append(env.world.get_state())
        states.append(step.state)
        if step.is_terminal:
            env.reset()

    for state, world_state in zip(states, world_states):
        env.set_state(state)
        assert env.world.get_state() == world_state


# Test cases
def test_reward():
    env = LLE.from_str(
        """
    S0 G .
    .  . X
    """
    ).build()
    env.reset()
    assert env.step([Action.EAST.value]).reward == REWARD_GEM
    assert env.step([Action.EAST.value]).reward == 0.0
    assert env.step([Action.SOUTH.value]).reward == REWARD_EXIT + REWARD_DONE


def test_reward_death():
    env = LLE.from_str(
        """
    S0 L0S X
    S1  .  X
    """
    ).build()
    env.reset()
    step = env.step([Action.STAY.value, Action.EAST.value])
    assert step.reward == REWARD_DEATH
    assert step.done


def test_reward_collect_and_death():
    env = LLE.from_str(
        """
    S0 L0S X
    S1  G  X
    """
    ).build()
    env.reset()
    step = env.step([Action.STAY.value, Action.EAST.value])
    assert step.reward.item() == REWARD_DEATH
    assert step.done


def test_multi_objective_rewards():
    env = (
        LLE.from_str(
            """
    S0 G .
    .  . X
    """,
        )
        .multi_objective()
        .build()
    )
    env.reset()
    # Collect the gem
    reward = env.step([Action.EAST.value]).reward
    assert reward[MultiObjective.RW_GEM_IDX] == 1.0
    for idx in [MultiObjective.RW_EXIT_IDX, MultiObjective.RW_DONE_IDX, MultiObjective.RW_DEATH_IDX]:
        assert reward[idx] == 0

    # Step east
    assert np.all(env.step([Action.EAST.value]).reward == 0.0)
    # Finish the level
    step = env.step([Action.SOUTH.value])
    assert step.done
    assert step.reward[MultiObjective.RW_EXIT_IDX] == REWARD_EXIT
    assert step.reward[MultiObjective.RW_DONE_IDX] == REWARD_DONE
    for idx in [MultiObjective.RW_GEM_IDX, MultiObjective.RW_DEATH_IDX]:
        assert step.reward[idx] == 0


def test_multi_objective_death():
    env = (
        LLE.from_str(
            """
    S0 L0S X
    S1  G  X
    """,
        )
        .multi_objective()
        .build()
    )
    env.reset()
    reward = env.step([Action.STAY.value, Action.EAST.value]).reward
    assert reward[MultiObjective.RW_DEATH_IDX] == REWARD_DEATH
    for idx in [MultiObjective.RW_GEM_IDX, MultiObjective.RW_EXIT_IDX, MultiObjective.RW_DONE_IDX]:
        assert reward[idx] == 0


def test_seed():
    LLE.level(1).build().seed(0)
    LLE.level(1).multi_objective().build().seed(0)


def test_env_name():
    for level in range(1, 7):
        env = LLE.level(level).build()
        assert env.name == f"LLE-lvl{level}"

        env = LLE.level(level).multi_objective().build()
        assert env.name == f"LLE-lvl{level}-MO"

    env = LLE.from_str("S0 X").build()
    assert env.name == "LLE"

    env = LLE.from_str("S0 X").multi_objective().build()
    assert env.name == "LLE-MO"

    with tempfile.NamedTemporaryFile(mode="w+", delete=True) as temp_file:
        temp_file.write("S0 X")
        temp_file.flush()

        env = LLE.from_file(temp_file.name).build()
        base = os.path.basename(temp_file.name)
        assert env.name == f"LLE-{base}"


def test_pbrs_reset_between_two_episodes():
    """
    We go south (and receive the shaped reward for crossing the laser) then terminate the episode.
    We then chek that we get the same reward when doing the same actions, after reset.
    """
    ACTIONS = [
        [Action.SOUTH.value],
        [Action.EAST.value],
        [Action.SOUTH.value],
        [Action.WEST.value],
    ]
    SHAPED_REWARD = 1.0
    EXPECTED_REWARDS = [
        SHAPED_REWARD,
        0.0,
        0.0,
        REWARD_EXIT + REWARD_DONE,
    ]
    env = (
        LLE.from_str("""
                       S0 .  .
                       .  . L0W
                       X  .  .""")
        .pbrs(reward_value=SHAPED_REWARD, gamma=1.0)
        .build()
    )

    for _ in range(5):
        env.reset()
        for action, expected_reward in zip(ACTIONS, EXPECTED_REWARDS):
            reward = env.step(action).reward
            assert reward == expected_reward


def test_pbrs_not_all_lasers():
    env = (
        LLE.from_str("""
                       S0 .  .
                       .  . L0W
                       .  . L0W
                       X  .  .""")
        .pbrs(lasers_to_reward=[(1, 2)])
        .build()
    )
    env.reset()
    assert env.extras_shape == (1,)
