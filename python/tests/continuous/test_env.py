"""Tests of the `CLLE` environment (round A, §4): spaces and shapes, `step` with `action_repeat`,
`done` and `info`, `reset`, `get_state`/`set_state`, copies, and the deferred features."""

import copy
import pickle

import lle.continuous as clle
import numpy as np
import pytest
from lle.continuous import CLLE, ContinuousWorld, ContinuousWorldState
from lle.env.reward_strategy import REWARD_DEATH, REWARD_DONE, REWARD_EXIT, REWARD_GEM
from marlenv import MARLEnv

from .common import CORRIDOR, EAST, GEM_THEN_EXIT, INTO_VOID, RANDOM_STARTS, SOUTH, STILL, TWO_AGENTS, WEST, xy


def test_env_is_a_marlenv_with_a_continuous_joint_action_space():
    env = clle.level(6).build()
    assert isinstance(env, CLLE)
    assert isinstance(env, MARLEnv)
    assert env.n_agents == 4
    space = env.action_space
    assert not space.is_discrete
    assert space.shape == (4, 2)
    assert np.all(space.low == -1.0)
    assert np.all(space.high == 1.0)


def test_shapes_are_known_before_reset():
    # Level 6: 4 agents with blocks of 6 + 4 values, 4 gems and 3 sources.
    env = clle.level(6).build()
    world = ContinuousWorld.level(6)
    assert env.observation_shape == (world.observation_size,) == (47,)
    assert env.state_shape == (world.state_size,) == (47,)
    assert env.extras_shape == (0,)


def test_reset_returns_the_world_observations_and_state():
    env = clle.from_str(TWO_AGENTS).build()
    obs, state = env.reset()
    world = ContinuousWorld(TWO_AGENTS)
    assert obs.data.dtype == np.float32
    assert obs.data.shape == (2, world.observation_size)
    np.testing.assert_array_equal(obs.data, np.stack([world.observation(0), world.observation(1)]))
    assert obs.extras.shape == (2, 0)
    np.testing.assert_array_equal(state.data, world.state_vector())
    np.testing.assert_array_equal(env.get_state().data, world.state_vector())
    np.testing.assert_array_equal(env.get_observation().data, obs.data)


def test_step_repeats_the_action_for_action_repeat_ticks():
    # 10 ticks at 0.1 cell per tick: from x = 0.5 to x = 1.5.
    env = clle.from_str(CORRIDOR).build()
    env.reset()
    step = env.step(np.array([EAST], dtype=np.float32))
    assert step.info["n_ticks"] == 10
    np.testing.assert_allclose(xy(env.world.agents_positions), [(1.5, 0.5)], rtol=1e-6)
    assert step.reward.tolist() == [0.0]
    assert not step.done
    np.testing.assert_array_equal(step.obs.data, env.get_observation().data)
    np.testing.assert_array_equal(step.state.data, env.world.state_vector())


def test_step_rewards_every_event_of_the_repeat():
    # Full speed: gem at tick index 4, exit at tick index 14, then early stop.
    env = clle.from_str(GEM_THEN_EXIT).action_repeat(30).build()
    env.reset()
    step = env.step([EAST])
    assert step.reward.tolist() == [REWARD_GEM + REWARD_EXIT + REWARD_DONE]
    assert step.done
    assert step.info["n_ticks"] == 15
    assert step.info["gems_collected"] == 1
    assert step.info["exit_rate"] == 1.0
    assert step.info["joint_exit"] == 1
    assert step.info["has-arrived-0"]
    assert step.info["is-alive-0"]


def test_death_ends_the_episode():
    # The disc overlaps the void at tick index 2 (x = 0.8 > 1 - r): early stop after 3 ticks.
    env = clle.from_str(INTO_VOID).build()
    env.reset()
    step = env.step([EAST])
    assert step.reward.tolist() == [REWARD_DEATH]
    assert step.done
    assert step.info["n_ticks"] == 3
    assert not step.info["is-alive-0"]
    assert not step.info["has-arrived-0"]
    assert step.info["exit_rate"] == 0.0
    assert step.info["joint_exit"] == 0
    assert step.info["gems_collected"] == 0


LASER_DEATH = """
S0 . . X
.  . . L1W
S1 . . X
"""
"""Agent 0 (colour 0) walks south into the beam of L1W, along y = 1.5: its disc touches the beam
when y > 1.5 - (r + beam_width / 2) = 1.15, i.e. at tick index 6 (y = 1.2). Agent 1 is safe."""


def test_laser_death_ends_the_episode():
    env = clle.from_str(LASER_DEATH).build()
    env.reset()
    step = env.step([SOUTH, STILL])
    assert step.reward.tolist() == [REWARD_DEATH]
    assert step.done
    assert step.info["n_ticks"] == 7
    assert not step.info["is-alive-0"]
    assert step.info["is-alive-1"]


def test_stepping_a_done_env_is_a_value_error():
    env = clle.from_str(INTO_VOID).build()
    env.reset()
    assert env.step([EAST]).done
    before = env.world.get_state()
    with pytest.raises(ValueError):
        env.step([EAST])
    assert env.world.get_state() == before


def test_episode_is_not_done_until_every_agent_has_arrived():
    env = clle.from_str("S0 X .\nS1 . X").action_repeat(20).build()
    env.reset()
    # Agent 0 arrives at tick index 7; agent 1 stays: no early stop.
    step = env.step([(0.7, 0.0), STILL])
    assert step.reward.tolist() == [REWARD_EXIT]
    assert not step.done
    assert step.info["n_ticks"] == 20
    assert step.info["exit_rate"] == 0.5
    assert step.info["joint_exit"] == 0
    assert step.info["has-arrived-0"] and not step.info["has-arrived-1"]
    # Agent 1 walks 1.5 cells east to the exit (2, 2): the episode ends.
    step = env.step([STILL, EAST])
    assert step.reward.tolist() == [REWARD_EXIT + REWARD_DONE]
    assert step.done
    assert step.info["exit_rate"] == 1.0
    assert step.info["joint_exit"] == 1


def test_sample_action_can_be_stepped():
    env = clle.level(6).build()
    env.reset()
    action = env.sample_action()
    assert np.shape(action) == (4, 2)
    step = env.step(action)
    assert step.obs.data.shape == (4, 47)
    assert 1 <= step.info["n_ticks"] <= 10


def test_reset_restarts_the_episode_and_the_rewards():
    env = clle.from_str(GEM_THEN_EXIT).action_repeat(30).build()
    for _ in range(3):
        obs, _ = env.reset()
        assert not env.done
        np.testing.assert_array_equal(obs.data, clle.from_str(GEM_THEN_EXIT).build().reset()[0].data)
        step = env.step([EAST])
        assert step.reward.tolist() == [REWARD_GEM + REWARD_EXIT + REWARD_DONE]
        assert env.done


def test_reset_with_a_seed_reproduces_random_starts():
    env = clle.from_str(RANDOM_STARTS).build()
    seen = set()
    for seed in range(10):
        obs, state = env.reset(seed=seed)
        again, again_state = env.reset(seed=seed)
        np.testing.assert_array_equal(obs.data, again.data)
        np.testing.assert_array_equal(state.data, again_state.data)
        seen.add(state.data.tobytes())
    assert len(seen) > 1


def test_set_state_accepts_a_world_state_and_resets_the_rewards():
    env = clle.from_str(GEM_THEN_EXIT).action_repeat(30).build()
    env.reset()
    initial = env.world.get_state()
    assert isinstance(initial, ContinuousWorldState)
    env.step([EAST])
    assert env.done
    env.set_state(initial)
    assert not env.done
    assert env.world.get_state() == initial
    np.testing.assert_array_equal(env.get_state().data, ContinuousWorld(GEM_THEN_EXIT).state_vector())
    # The reward strategy was reset: the DONE bonus is given again, the exit counted once.
    step = env.step([EAST])
    assert step.reward.tolist() == [REWARD_GEM + REWARD_EXIT + REWARD_DONE]


PARTIAL = """
S0 X . .
S1 . . X
.  . L1N .
"""
"""Agent 0 reaches the exit (0, 1) in the first step of `PARTIAL_FIRST`. Then agent 1 walks east
along row 1: it enters its own beam (L1N on (2, 2) covers (1, 2) and (0, 2)) in the first step of
`PARTIAL_SECOND`, and the exit (1, 3) in the second step."""
PARTIAL_FIRST = [(0.7, 0.0), STILL]
PARTIAL_SECOND = [STILL, EAST]


def partial_builder(reward: str):
    builder = clle.from_str(PARTIAL).action_repeat(20)
    if "multi" in reward:
        builder = builder.multi_objective()
    if "pbrs" in reward:
        builder = builder.pbrs(gamma=0.99, reward_value=0.5)
    return builder


def finish(env: CLLE):
    rewards = []
    while not env.done:
        rewards.append(env.step(PARTIAL_SECOND).reward)
    return rewards


@pytest.mark.parametrize("reward", ["single", "multi", "single-pbrs", "multi-pbrs"])
def test_set_state_after_a_partial_completion_keeps_the_completion_reward(reward: str):
    """An agent that arrived before the snapshot still counts: finishing the restored episode
    gives the same rewards (the DONE bonus included) as finishing it without interruption."""
    uninterrupted = partial_builder(reward).build()
    uninterrupted.reset()
    uninterrupted.step(PARTIAL_FIRST)
    snapshot = uninterrupted.world.get_state()
    assert list(snapshot.agents_arrived) == [True, False]
    copies = [copy.deepcopy(uninterrupted), pickle.loads(pickle.dumps(uninterrupted))]
    expected = finish(uninterrupted)
    assert len(expected) == 2

    restored = partial_builder(reward).build()
    restored.reset()
    # Another history before the restoration: agent 1 has already entered its beam.
    restored.step(PARTIAL_SECOND)
    restored.set_state(snapshot)
    assert not restored.done
    for env in [restored, *copies]:
        np.testing.assert_allclose(finish(env), expected, atol=1e-6)
    if reward == "single":
        assert [r.tolist() for r in expected] == [[0.0], [REWARD_EXIT + REWARD_DONE]]


def test_set_state_recomputes_done_from_the_world_flags():
    env = clle.from_str(TWO_AGENTS).build()
    env.reset()
    state = env.world.get_state()
    state.agents_arrived = [True, False]
    env.set_state(state)
    assert not env.done
    state.agents_arrived = [True, True]
    env.set_state(state)
    assert env.done
    state.agents_arrived = [False, False]
    state.agents_alive = [True, False]
    env.set_state(state)
    assert env.done
    state.agents_alive = [True, True]
    env.set_state(state)
    assert not env.done


def test_set_state_with_a_state_vector_is_not_implemented():
    env = clle.from_str(TWO_AGENTS).build()
    env.reset()
    with pytest.raises(NotImplementedError):
        env.set_state(env.get_state())


@pytest.mark.parametrize("duplicate", [copy.deepcopy, lambda env: pickle.loads(pickle.dumps(env))], ids=["deepcopy", "pickle"])
def test_copies_are_independent_envs(duplicate):
    env = clle.from_str(TWO_AGENTS).action_repeat(3).build()
    env.reset()
    env.step([(0.0, 1.0), (0.5, 0.5)])
    clone = duplicate(env)
    np.testing.assert_array_equal(clone.get_state().data, env.get_state().data)
    # Same dynamics.
    a = env.step([(0.3, 0.2), STILL])
    b = clone.step([(0.3, 0.2), STILL])
    np.testing.assert_array_equal(a.obs.data, b.obs.data)
    np.testing.assert_array_equal(a.reward, b.reward)
    assert a.done == b.done
    # Independent worlds.
    before = env.get_state().data.copy()
    clone.step([WEST, SOUTH])
    np.testing.assert_array_equal(env.get_state().data, before)
    assert clone.world is not env.world


@pytest.mark.parametrize("duplicate", [copy.deepcopy, lambda env: pickle.loads(pickle.dumps(env))], ids=["deepcopy", "pickle"])
def test_copies_keep_the_random_starts(duplicate):
    env = clle.from_str(RANDOM_STARTS).build()
    env.reset(seed=3)
    clone = duplicate(env)
    states = set()
    for _ in range(5):
        _, state = env.reset()
        _, clone_state = clone.reset()
        np.testing.assert_array_equal(clone_state.data, state.data)
        states.add(state.data.tobytes())
    assert len(states) > 1


def test_get_image_is_not_implemented():
    env = clle.from_str(TWO_AGENTS).build()
    env.reset()
    with pytest.raises(NotImplementedError):
        env.get_image()
