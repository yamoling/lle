"""Tests of the static features of the `STATE` observation (round B, §2.4).

With `static_features=True`, `state_vector` and `observation` append a static block after the
global block, in absolute coordinates for every agent:
- each exit's cell centre `(x/width, y/height)`, in exit order;
- each gem's centre, in gem order;
- each laser source's cell centre and a one-hot direction `[N, E, S, W]`, in laser id order.
"""

import lle.continuous as clle
import numpy as np
from lle.continuous import ContinuousWorld

from .common import EAST, SOUTH

MAP = """
S0  .   G X
.   L0S . .
S1  .   G X
L1E .   . .
"""
# Width 4, height 4. Exits (0, 3) and (2, 3); gems (0, 2) and (2, 2). L0S on (1, 1) has laser id 0
# and points south, L1E on (3, 0) has laser id 1 and points east. No start lies in a beam.
STATIC = [
    *[3.5 / 4, 0.5 / 4, 3.5 / 4, 2.5 / 4],  # exits (0.875, 0.125) and (0.875, 0.625)
    *[2.5 / 4, 0.5 / 4, 2.5 / 4, 2.5 / 4],  # gems (0.625, 0.125) and (0.625, 0.625)
    *[1.5 / 4, 1.5 / 4, 0, 0, 1, 0],  # laser 0: (0.375, 0.375), south
    *[0.5 / 4, 3.5 / 4, 0, 1, 0, 0],  # laser 1: (0.125, 0.875), east
]


def test_the_map_lists_its_elements_in_the_expected_order():
    """A precondition of the hand-written `STATIC`."""
    world = ContinuousWorld(MAP)
    assert [tuple(p)[:2] for p in world.exits_positions] == [(0, 3), (2, 3)]
    assert [tuple(p)[:2] for p in world.gems_positions] == [(0, 2), (2, 2)]
    assert sorted((laser_id, tuple(cell)[:2]) for cell, _, _, laser_id in world.laser_sources) == [(0, (1, 1)), (1, (3, 0))]


def test_state_vector_appends_the_static_block():
    world = ContinuousWorld(MAP)
    vector = world.state_vector(static_features=True)
    assert vector.dtype == np.float32
    np.testing.assert_array_equal(vector[: world.state_size], world.state_vector())
    np.testing.assert_allclose(vector[world.state_size :], STATIC, rtol=1e-6)


def test_every_observation_appends_the_same_absolute_static_block():
    world = ContinuousWorld(MAP)
    for agent in range(2):
        obs = world.observation(agent, static_features=True)
        np.testing.assert_array_equal(obs[: world.observation_size], world.observation(agent))
        np.testing.assert_allclose(obs[world.observation_size :], STATIC, rtol=1e-6)


def test_static_features_are_off_by_default():
    world = ContinuousWorld(MAP)
    assert world.state_vector().shape == (world.state_size,)
    np.testing.assert_array_equal(world.state_vector(static_features=False), world.state_vector())
    np.testing.assert_array_equal(world.observation(1, static_features=False), world.observation(1))


def test_static_block_does_not_change_with_the_state():
    # Agent 0 collects the gem (0, 2) and agent 1 walks south; source 0 is disabled.
    world = ContinuousWorld(MAP)
    world.step([EAST, SOUTH], 15)
    assert world.gems_collected == [True, False]
    state = world.get_state()
    state.sources_enabled = [False, True]
    world.set_state(state)
    np.testing.assert_allclose(world.state_vector(static_features=True)[world.state_size :], STATIC, rtol=1e-6)


def test_static_block_size_counts_exits_gems_and_sources():
    # Level 6: 4 exits and 4 gems (2 values each), 3 sources (6 values each).
    world = ContinuousWorld.level(6)
    assert (len(world.exits_positions), len(world.gems_positions), len(world.laser_sources)) == (4, 4, 3)
    assert world.state_vector(static_features=True).shape == (world.state_size + 34,)
    assert world.observation(3, static_features=True).shape == (world.observation_size + 34,)


def test_env_includes_the_static_features_in_its_observations_and_state():
    env = clle.from_str(MAP).include_static_features().build()
    world = ContinuousWorld(MAP)
    size = world.state_size + len(STATIC)
    assert env.observation_shape == (size,)
    assert env.state_shape == (size,)
    obs, state = env.reset()
    assert obs.data.shape == (2, size)
    np.testing.assert_array_equal(obs.data, np.stack([world.observation(i, static_features=True) for i in range(2)]))
    np.testing.assert_array_equal(state.data, world.state_vector(static_features=True))


def test_env_without_static_features_keeps_the_round_a_shapes():
    env = clle.from_str(MAP).build()
    world = ContinuousWorld(MAP)
    assert env.observation_shape == (world.observation_size,)
    assert env.state_shape == (world.state_size,)
