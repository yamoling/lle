"""Tests of the rewards of `CLLE` (round A, §4): the reward strategies of `lle.env.reward_strategy`
are reused unchanged and fed every event of the repeat, and PBRS maps an agent to the cell that
contains its centre."""

import copy
import pickle

import lle.continuous as clle
import numpy as np
import pytest
from lle.continuous import ContinuousWorld
from lle.env.reward_strategy import REWARD_DONE, REWARD_EXIT, REWARD_GEM, MultiObjective, SingleObjective

from .common import EAST, GEM_THEN_EXIT, NORTH, SOUTH

PBRS_MAP = """
S0  . .
.   . L0W
X   . .
L0E . .
"""
# One agent (colour 0) at (0.5, 0.5); both sources have its colour, so it never dies. Laser 0 is
# L0W on (1, 2): beam cells (1, 1) and (1, 0). Laser 1 is L0E on (3, 0): beam cells (3, 1), (3, 2).
# With action_repeat = 10, every step moves the agent by exactly one cell, along the cells
# (0, 1) -> (1, 1) [laser 0] -> (2, 1) -> (3, 1) [laser 1].
PBRS_PATH = [EAST, SOUTH, SOUTH, SOUTH]


@pytest.mark.parametrize(
    "multi_objective, strategy",
    [(False, SingleObjective), (True, MultiObjective)],
    ids=["single", "multi"],
)
def test_rewards_equal_the_strategy_fed_the_events_of_the_repeat(multi_objective: bool, strategy: type):
    """Random rollouts on level 6, against a twin world stepped by hand with the same actions."""
    builder = clle.level(6)
    if multi_objective:
        builder = builder.multi_objective()
    env = builder.build()
    twin = ContinuousWorld.level(6)
    expected_strategy = strategy(4)
    rng = np.random.default_rng(0)
    n_events = 0
    for _ in range(5):
        env.reset()
        twin.reset()
        expected_strategy.reset()
        for _ in range(30):
            # Drift southwards so that agents reach the L0E beam (row 4) and die or block it. The
            # actions are float32 values, so whatever float type the env forwards, both worlds
            # receive exactly the same actions.
            actions = np.column_stack([rng.uniform(-1, 1, 4), rng.uniform(-0.5, 1, 4)]).astype(np.float32)
            step = env.step(actions)
            outcome = twin.step(actions, 10)
            n_events += len(outcome.events)
            np.testing.assert_array_equal(step.state.data, twin.state_vector())
            np.testing.assert_array_equal(step.reward, expected_strategy.compute_reward(outcome.events))
            if step.done:
                break
    assert n_events > 0, "the rollouts never produced an event"


def test_multi_objective_rewards_every_event_of_the_repeat():
    # Full speed: gem at tick index 4, exit at tick index 14, in the same step.
    env = clle.from_str(GEM_THEN_EXIT).action_repeat(30).multi_objective().build()
    env.reset()
    assert env.reward_space.shape == (4,)
    step = env.step([EAST])
    # [gem, exit, death, done]
    assert step.reward.tolist() == [REWARD_GEM, REWARD_EXIT, 0.0, REWARD_DONE]


def test_pbrs_rewards_entering_every_laser_by_default():
    # Two (agent, laser) pairs: the potential starts at 2 * 0.5 = 1.0.
    env = clle.from_str(PBRS_MAP).pbrs(gamma=0.99, reward_value=0.5).build()
    env.reset()
    rewards = [env.step([action]).reward[0] for action in PBRS_PATH]
    # gamma * previous potential - current potential
    expected = [
        0.99 * 1.0 - 1.0,  # (0, 1): no laser
        0.99 * 1.0 - 0.5,  # (1, 1): laser 0 reached
        0.99 * 0.5 - 0.5,  # (2, 1)
        0.99 * 0.5 - 0.0,  # (3, 1): laser 1 reached
    ]
    assert rewards == pytest.approx(expected, abs=1e-6)
    assert tuple(env.world.agents_cells[0]) == (3, 1, 0)


def test_pbrs_rewards_only_the_selected_lasers():
    env = clle.from_str(PBRS_MAP).pbrs(gamma=0.99, reward_value=0.5, lasers_to_reward=[1]).build()
    env.reset()
    rewards = [env.step([action]).reward[0] for action in PBRS_PATH]
    expected = [0.99 * 0.5 - 0.5] * 3 + [0.99 * 0.5]
    assert rewards == pytest.approx(expected, abs=1e-6)


def test_pbrs_potential_is_reset_with_the_env():
    env = clle.from_str(PBRS_MAP).pbrs(gamma=0.99, reward_value=0.5).build()
    for _ in range(2):
        env.reset()
        rewards = [env.step([action]).reward[0] for action in PBRS_PATH[:2]]
        assert rewards == pytest.approx([0.99 - 1.0, 0.99 - 0.5], abs=1e-6)


def test_pbrs_with_multi_objective_adds_an_objective():
    env = clle.from_str(PBRS_MAP).multi_objective().pbrs(gamma=0.99, reward_value=0.5).build()
    env.reset()
    assert env.reward_space.shape == (5,)
    first = env.step([EAST]).reward
    second = env.step([SOUTH]).reward
    np.testing.assert_allclose(first, [0, 0, 0, 0, 0.99 - 1.0], atol=1e-6)
    np.testing.assert_allclose(second, [0, 0, 0, 0, 0.99 - 0.5], atol=1e-6)


# Reach laser 0, leave it, come back, leave it again, then reach laser 1.
PBRS_DETOUR = [EAST, SOUTH, SOUTH, NORTH, SOUTH, SOUTH]
PBRS_DETOUR_REWARDS = [0.99 - 1.0, 0.99 - 0.5, 0.99 * 0.5 - 0.5, 0.99 * 0.5 - 0.5, 0.99 * 0.5 - 0.5, 0.99 * 0.5]


@pytest.mark.parametrize("copy_after", [2, 3, 4])
@pytest.mark.parametrize("duplicate", [copy.deepcopy, lambda env: pickle.loads(pickle.dumps(env))], ids=["deepcopy", "pickle"])
def test_pbrs_copies_keep_the_reached_lasers(duplicate, copy_after: int):
    """A copy made after laser 0 was reached keeps that history and the previous potential: it
    rewards the rest of the detour like an uninterrupted run, and so does the original."""
    env = clle.from_str(PBRS_MAP).pbrs(gamma=0.99, reward_value=0.5).build()
    env.reset()
    rewards = [env.step([action]).reward[0] for action in PBRS_DETOUR[:copy_after]]
    clone = duplicate(env)
    clone_rewards = [clone.step([action]).reward[0] for action in PBRS_DETOUR[copy_after:]]
    env_rewards = [env.step([action]).reward[0] for action in PBRS_DETOUR[copy_after:]]
    assert rewards + clone_rewards == pytest.approx(PBRS_DETOUR_REWARDS, abs=1e-6)
    assert rewards + env_rewards == pytest.approx(PBRS_DETOUR_REWARDS, abs=1e-6)
    assert tuple(clone.world.agents_cells[0]) == tuple(env.world.agents_cells[0]) == (3, 1, 0)


@pytest.mark.parametrize("duplicate", [copy.deepcopy, lambda env: pickle.loads(pickle.dumps(env))], ids=["deepcopy", "pickle"])
def test_pbrs_copies_read_their_own_world(duplicate):
    """If the copied shaping read the original world, the clone would see the agent of the original
    (still on its start cell) and miss laser 0."""
    env = clle.from_str(PBRS_MAP).pbrs(gamma=0.99, reward_value=0.5).build()
    env.reset()
    clone = duplicate(env)
    clone_rewards = [clone.step([action]).reward[0] for action in PBRS_PATH[:2]]
    assert clone_rewards == pytest.approx([0.99 - 1.0, 0.99 - 0.5], abs=1e-6)
    # The original potential did not move with the clone.
    env_rewards = [env.step([action]).reward[0] for action in PBRS_PATH[:2]]
    assert env_rewards == pytest.approx([0.99 - 1.0, 0.99 - 0.5], abs=1e-6)
