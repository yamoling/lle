from lle import LLE, Action, WorldState
from lle.env.core import REWARD_DEATH, REWARD_GEM, REWARD_EXIT, REWARD_DONE
from lle.env.multi_objective import RW_DEATH_IDX, RW_GEM_IDX, RW_EXIT_IDX, RW_DONE_IDX
import numpy as np


def test_void_reward():
    env = LLE.from_str("S0 V X").single_objective()
    env.reset()
    assert env.step([Action.EAST.value])[1] == REWARD_DEATH
    assert env.done


def test_collect_reward():
    env = LLE.from_str(
        """S0 X . .
.  . . .
G  . . ."""
    ).single_objective()
    env.reset()
    env.step([Action.SOUTH.value])
    reward = env.step([Action.SOUTH.value])[1]
    assert reward == REWARD_GEM


def test_time_reward():
    env = LLE.from_str(
        """
    . .  . X
    . S0 . .
    . .  . ."""
    ).single_objective()
    env.reset()
    for action in Action.ALL:
        _obs, reward, *_ = env.step([action.value])
        assert reward == 0


def test_finish_reward():
    env = LLE.from_str(
        """@ @ @  @ @ @
@ . .  . . @
@ . S0 . . @
@ . .  X . @
@ @ @  @ @ @"""
    ).single_objective()
    env.reset()
    env.step([Action.EAST.value])
    reward = env.step([Action.SOUTH.value])[1]
    assert reward == REWARD_DONE + REWARD_EXIT


def test_arrive_reward_only_once():
    """Some kind of adversarial game where only one agent can move at a time."""
    env = LLE.from_str(
        """
    S0 . G
    S1 X X
""",
    ).single_objective()
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
        r = env.step([a.value for a in action])[1]
        assert r == reward


def test_reward_after_reset():
    env = LLE.from_str(
        """
    S0 X . .
    .  . . .
    G  . . .
    """
    ).single_objective()

    def play():
        """Collect the gem and finish the game. Check that the reward is is correct when collecting it."""
        env.reset()
        env.step([Action.SOUTH.value])
        reward = env.step([Action.SOUTH.value])[1]
        assert reward == REWARD_GEM
        assert not env.done
        r = env.step([Action.NORTH.value])[1]
        assert r == 0
        r = env.step([Action.NORTH.value])[1]
        assert r == 0
        reward = env.step([Action.EAST.value])[1]
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
    ).single_objective()
    env.reset()
    state = WorldState([(0, 1), (1, 1)], [False])
    env.set_state(state)
    assert env.step([Action.EAST.value, Action.STAY.value])[1] == REWARD_GEM


def test_reward_set_state_all_arrived():
    env = LLE.from_str(
        """
    S0 . G
    S1 X X""",
    ).single_objective()
    env.reset()
    state = WorldState([(0, 2), (1, 1)], [True])
    env.set_state(state)
    r = env.step([Action.SOUTH.value, Action.STAY.value])[1]
    assert r == REWARD_DONE + REWARD_EXIT


def test_set_state():
    env = LLE.from_str("S0 G X").single_objective()
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
    ).single_objective()
    env.reset()
    assert env.step([Action.EAST.value])[1] == REWARD_GEM
    assert env.step([Action.EAST.value])[1] == 0.0
    assert env.step([Action.SOUTH.value])[1] == REWARD_EXIT + REWARD_DONE


def test_reward_death():
    env = LLE.from_str(
        """
    S0 L0S X
    S1  .  X
    """
    ).single_objective()
    env.reset()
    _, r, _, _, _ = env.step([Action.STAY.value, Action.EAST.value])
    assert r == REWARD_DEATH
    assert env.done


def test_reward_collect_and_death():
    env = LLE.from_str(
        """
    S0 L0S X
    S1  G  X
    """
    ).single_objective()
    env.reset()
    _, r, _, _, _ = env.step([Action.STAY.value, Action.EAST.value])
    assert r == REWARD_DEATH
    assert env.done


def test_multi_objective_rewards():
    env = LLE.from_str(
        """
    S0 G .
    .  . X
    """,
    ).multi_objective()
    indices = [RW_GEM_IDX, RW_EXIT_IDX, RW_DONE_IDX, RW_DEATH_IDX]
    env.reset()
    # Collect the gem
    reward = env.step([Action.EAST.value])[1]
    assert reward[RW_GEM_IDX] == REWARD_GEM
    for idx in indices:
        if idx != RW_GEM_IDX:
            assert reward[idx] == 0

    # Step east
    assert np.all(env.step([Action.EAST.value])[1] == 0.0)
    # Finish the level
    _, reward, done, *_ = env.step([Action.SOUTH.value])
    assert done
    assert reward[RW_EXIT_IDX] == REWARD_EXIT
    assert reward[RW_DONE_IDX] == REWARD_DONE
    for idx in indices:
        if idx not in [RW_EXIT_IDX, RW_DONE_IDX]:
            assert reward[idx] == 0


def test_multi_objective_death():
    env = LLE.from_str(
        """
    S0 L0S X
    S1  G  X
    """,
    ).multi_objective()
    env.reset()
    reward = env.step([Action.STAY.value, Action.EAST.value])[1]
    assert reward[RW_DEATH_IDX] == REWARD_DEATH
    for idx in [RW_GEM_IDX, RW_EXIT_IDX, RW_DONE_IDX]:
        assert reward[idx] == 0
