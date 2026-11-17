"""Tests of the round B builder options and observation types of `CLLE` (round B, §2, §5).

On level 6 (width 13, height 12, 4 agents, n_colours = 4):
- `STATE`: `(47,)` (round A);
- `LAYERED`: `(2 · 4 + 4, 12 · res, 13 · res)`, with `res = 4` by default: `(12, 48, 52)`;
- `LIDAR`: `(n_rays · (5 + 4) + 6 + 4,)`, with 16 rays by default: `(154,)`;
- `RGB_IMAGE`: `(12 · 32, 13 · 32, 3)` = `(384, 416, 3)`.
"""

import copy
import math
import pickle
from typing import Any

import lle.continuous as clle
import numpy as np
import pytest
from lle.continuous import ContinuousWorld, ObservationType

from .common import CORRIDOR, EAST, SOUTH, TWO_AGENTS, WEST


@pytest.mark.parametrize(
    "alias, name",
    [("state", "STATE"), ("layered", "LAYERED"), ("lidar", "LIDAR"), ("rgb", "RGB_IMAGE"), ("rgb-image", "RGB_IMAGE")],
)
def test_observation_type_aliases(alias: str, name: str):
    # "rgb-image" is the name of LLE's `ObservationType.RGB_IMAGE`, "rgb" the one of the contract.
    member = ObservationType[name]
    assert ObservationType.from_str(alias) is member
    env = clle.from_str(TWO_AGENTS).obs_type(alias).build()  # type: ignore[arg-type]
    assert env.obs_type == member.name


@pytest.mark.parametrize(
    "configure, obs_shape",
    [
        (lambda b: b.obs_type("state"), (47,)),
        (lambda b: b.obs_type("layered"), (12, 48, 52)),
        (lambda b: b.obs_type("layered").layered_resolution(1), (12, 12, 13)),
        (lambda b: b.obs_type("layered").layered_resolution(3), (12, 36, 39)),
        (lambda b: b.obs_type("lidar"), (154,)),
        (lambda b: b.obs_type("lidar").lidar(n_rays=8), (8 * 9 + 10,)),
        (lambda b: b.obs_type("lidar").lidar(n_rays=1, range=2.0), (9 + 10,)),
        (lambda b: b.obs_type(ObservationType.RGB_IMAGE), (384, 416, 3)),
    ],
    ids=["state", "layered", "layered-res-1", "layered-res-3", "lidar", "lidar-8-rays", "lidar-1-ray", "rgb"],
)
def test_observation_shapes_are_known_before_reset(configure: Any, obs_shape: tuple[int, ...]):
    env = configure(clle.level(6)).build()
    assert env.observation_shape == obs_shape
    obs, _ = env.reset()
    assert obs.data.dtype == np.float32
    assert obs.data.shape == (4, *obs_shape)


@pytest.mark.parametrize(
    "configure, state_shape",
    [
        (lambda b: b.state_type("state"), (47,)),
        (lambda b: b.state_type("layered"), (12, 48, 52)),
        (lambda b: b.state_type("layered").layered_resolution(2), (12, 24, 26)),
        (lambda b: b.state_type("rgb"), (384, 416, 3)),
    ],
    ids=["state", "layered", "layered-res-2", "rgb"],
)
def test_state_shapes_are_known_before_reset(configure: Any, state_shape: tuple[int, ...]):
    env = configure(clle.level(6)).build()
    assert env.state_shape == state_shape
    _, state = env.reset()
    assert state.data.shape == state_shape
    assert env.get_state().data.shape == state_shape


def test_layered_observations_are_the_world_layered_array_for_every_agent():
    env = clle.from_str(TWO_AGENTS).obs_type("layered").layered_resolution(2).build()
    env.reset()
    step = env.step([EAST, SOUTH])
    expected = env.world.layered(2)
    for agent in range(2):
        np.testing.assert_array_equal(step.obs.data[agent], expected)


def test_lidar_observations_are_the_world_lidar_of_each_agent():
    env = clle.from_str(TWO_AGENTS).obs_type("lidar").lidar(n_rays=6, range=3.0).build()
    env.reset()
    step = env.step([SOUTH, EAST])
    for agent in range(2):
        np.testing.assert_array_equal(step.obs.data[agent], env.world.lidar(agent, n_rays=6, range=3.0))


def test_rgb_observations_are_the_frame_for_every_agent():
    env = clle.from_str(TWO_AGENTS).obs_type("rgb").build()
    obs, _ = env.reset()
    frame = env.world.get_image().astype(np.float32)
    assert obs.data.shape == (2, 64, 96, 3)
    for agent in range(2):
        np.testing.assert_array_equal(obs.data[agent], frame)


@pytest.mark.parametrize("state_type", ["state", "layered", "rgb"])
def test_states_are_the_world_arrays(state_type: str):
    env = clle.from_str(TWO_AGENTS).state_type(state_type).layered_resolution(3).build()  # type: ignore[arg-type]
    env.reset()
    step = env.step([EAST, SOUTH])
    world = env.world
    expected = {
        "state": world.state_vector(),
        "layered": world.layered(3),
        "rgb": world.get_image(),
    }[state_type]
    np.testing.assert_array_equal(step.state.data, expected)
    np.testing.assert_array_equal(env.get_state().data, expected)


def test_lidar_state_type_is_a_value_error():
    # The lidar is a per-agent view: it cannot be a state. Raising in the setter or in `build()` is fine.
    with pytest.raises(ValueError):
        clle.from_str(TWO_AGENTS).state_type("lidar").build()  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        clle.from_str(TWO_AGENTS).state_type(ObservationType.LIDAR).build()


def test_unknown_observation_type_is_still_a_value_error():
    with pytest.raises(ValueError):
        clle.from_str(TWO_AGENTS).obs_type("perspective").build()  # type: ignore[arg-type]


@pytest.mark.parametrize(
    "configure",
    [
        lambda b: b.obs_type("layered").layered_resolution(0),
        lambda b: b.obs_type("layered").layered_resolution(-2),
        lambda b: b.state_type("layered").layered_resolution(0),
        lambda b: b.obs_type("lidar").lidar(n_rays=0),
        lambda b: b.obs_type("lidar").lidar(n_rays=-4),
        lambda b: b.obs_type("lidar").lidar(range=0.0),
        lambda b: b.obs_type("lidar").lidar(range=-1.0),
        lambda b: b.obs_type("lidar").lidar(range=math.inf),
        lambda b: b.obs_type("lidar").lidar(range=math.nan),
    ],
    ids=[
        "zero-res",
        "negative-res",
        "zero-res-for-the-state",
        "no-ray",
        "negative-rays",
        "zero-range",
        "negative-range",
        "infinite-range",
        "nan-range",
    ],
)
def test_invalid_observation_options_raise_at_build(configure: Any):
    # As in round A, the setters store raw values and `build()` validates them.
    builder = configure(clle.from_str(TWO_AGENTS))
    with pytest.raises(ValueError):
        builder.build()


@pytest.mark.parametrize("obs_type", ["state", "layered", "lidar", "rgb"])
@pytest.mark.parametrize("duplicate", ["deepcopy", "pickle"])
def test_copies_of_envs_are_independent_for_every_observation_type(obs_type: str, duplicate: str):
    env = clle.from_str(CORRIDOR).obs_type(obs_type).state_type("layered").layered_resolution(2).lidar(n_rays=4).build()  # type: ignore[arg-type]
    env.reset()
    env.step([EAST])
    other = copy.deepcopy(env) if duplicate == "deepcopy" else pickle.loads(pickle.dumps(env))
    assert other.observation_shape == env.observation_shape
    assert other.state_shape == env.state_shape
    np.testing.assert_array_equal(other.get_observation().data, env.get_observation().data)
    np.testing.assert_array_equal(other.get_state().data, env.get_state().data)
    # Moving the copy leaves the original where it was, and both observe their own world.
    before = env.get_observation().data
    step = other.step([WEST])
    np.testing.assert_array_equal(env.get_observation().data, before)
    assert not np.array_equal(step.obs.data, before)
    step = env.step([WEST])
    np.testing.assert_array_equal(step.obs.data, other.get_observation().data)


def test_copies_keep_the_randomized_colours_and_static_features():
    env = clle.from_str(TWO_AGENTS).randomize_lasers().include_static_features().build()
    env.reset(seed=1)
    for other in (copy.deepcopy(env), pickle.loads(pickle.dumps(env))):
        assert other.world.laser_sources == env.world.laser_sources
        assert other.observation_shape == env.observation_shape
        np.testing.assert_array_equal(other.get_observation().data, env.get_observation().data)
        np.testing.assert_array_equal(other.get_state().data, env.get_state().data)


def test_default_observation_options_reach_the_world():
    # Default resolution 4, default lidar of 16 rays with a range of 5.
    world = ContinuousWorld(TWO_AGENTS)
    layered = clle.from_str(TWO_AGENTS).obs_type("layered").build()
    assert layered.observation_shape == tuple(world.layered_shape(4))
    lidar = clle.from_str(TWO_AGENTS).obs_type("lidar").build()
    assert lidar.observation_shape == (world.lidar_size(16),)
    obs, _ = lidar.reset()
    np.testing.assert_array_equal(obs.data[1], world.lidar(1, n_rays=16, range=5.0))
