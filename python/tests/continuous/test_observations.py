"""Tests of the STATE observation and state vector of `ContinuousWorld` (round A, §2).

Layout of an agent block (`6 + n_colours` values):
`x/width, y/height, vx/max_speed, vy/max_speed, alive, arrived`, then a one-hot colour.
The global block follows the agent blocks: one flag per gem (1 = collected), then one flag per
laser source (1 = enabled), in laser order.

- `state_vector()`: every agent block in id order, with absolute coordinates, then the global block.
- `observation(i)`: the block of `i` (absolute), then the other agents in increasing id order, whose
  `x`/`y` only are relative to `i`: `(x_k - x_i)/width`, `(y_k - y_i)/height`. Then the global block.

The expected vectors below are derived by hand on `MAP`.
"""

import numpy as np
import pytest
from lle.continuous import ContinuousWorld, PhysicsParams

from .common import SAME_COLOUR

MAP = """
S0 .   S1 . X
G  .   .  G X
S2 L1N .  . X
.  .   .  . L2W
"""
# Width 5, height 4. Agents 0, 1 and 2 (colours 0, 1, 2) start at (0.5, 0.5), (2.5, 0.5) and
# (0.5, 2.5). Gems on (1, 0) and (1, 3). Laser 0 is L1N on (2, 1) (its beam runs along x = 1.5,
# 1.0 away from agents 0 and 1), laser 1 is L2W on (3, 4) (along y = 3.5). n_colours = 3, so an
# agent block has 9 values, and the state has 3 * 9 + 2 + 2 = 31.
W, H = 5.0, 4.0
ONE_HOT = {0: [1, 0, 0], 1: [0, 1, 0], 2: [0, 0, 1]}


def block(x: float, y: float, colour: int, vx: float = 0.0, vy: float = 0.0, alive: bool = True, arrived: bool = False):
    """The expected block of an agent, where `x`, `y`, `vx` and `vy` are already normalized."""
    return [x, y, vx, vy, float(alive), float(arrived), *ONE_HOT[colour]]


def assert_vector(actual: np.ndarray, expected: list[float]):
    """
    Check the dtype, shape and values of an observation or state vector. The values are computed
    in `f64` then exported as `f32`, hence the relative tolerance.

    @ai-generated
    """
    assert isinstance(actual, np.ndarray)
    assert actual.dtype == np.float32
    assert actual.shape == (len(expected),)
    np.testing.assert_allclose(actual, np.array(expected, dtype=np.float32), rtol=1e-6, atol=1e-7)


def test_sizes():
    world = ContinuousWorld(MAP)
    assert world.state_size == 31
    assert world.observation_size == 31


def test_n_colours_sets_the_one_hot_size():
    # Both agents and the only source have colour 0: blocks of 6 + 1 values, no gem, one source.
    world = ContinuousWorld(SAME_COLOUR)
    assert world.state_size == 2 * 7 + 0 + 1
    vector = world.state_vector()
    assert vector[6] == 1.0 and vector[13] == 1.0


def test_state_vector_at_reset():
    world = ContinuousWorld(MAP)
    assert_vector(
        world.state_vector(),
        [
            *block(0.5 / W, 0.5 / H, 0),  # 0.1, 0.125
            *block(2.5 / W, 0.5 / H, 1),  # 0.5, 0.125
            *block(0.5 / W, 2.5 / H, 2),  # 0.1, 0.625
            *[0, 0],  # no gem collected
            *[1, 1],  # both sources enabled
        ],
    )


@pytest.mark.parametrize(
    "agent, expected",
    [
        (
            0,
            [
                *block(0.1, 0.125, 0),
                *block(2.0 / W, 0.0, 1),  # agent 1, relative: (2.5 - 0.5) / 5 = 0.4
                *block(0.0, 2.0 / H, 2),  # agent 2, relative: (2.5 - 0.5) / 4 = 0.5
                *[0, 0, 1, 1],
            ],
        ),
        (
            1,
            [
                *block(0.5, 0.125, 1),
                *block(-2.0 / W, 0.0, 0),  # agent 0: (0.5 - 2.5) / 5 = -0.4
                *block(-2.0 / W, 2.0 / H, 2),  # agent 2: -0.4, (2.5 - 0.5) / 4 = 0.5
                *[0, 0, 1, 1],
            ],
        ),
        (
            2,
            [
                *block(0.1, 0.625, 2),
                *block(0.0, -2.0 / H, 0),  # agent 0: 0, (0.5 - 2.5) / 4 = -0.5
                *block(2.0 / W, -2.0 / H, 1),  # agent 1: 0.4, -0.5
                *[0, 0, 1, 1],
            ],
        ),
    ],
)
def test_observation_is_ego_first_with_relative_positions(agent: int, expected: list[float]):
    world = ContinuousWorld(MAP)
    assert_vector(world.observation(agent), expected)


def test_flags_follow_the_world_state():
    # Agent 1 is dead where it stands, agent 2 is moved onto the exit (2, 4) and marked as arrived,
    # the second gem is collected and laser 1 is disabled.
    world = ContinuousWorld(MAP)
    state = world.get_state()
    state.positions = [(0.5, 0.5), (2.5, 0.5), (4.5, 2.5)]
    state.agents_alive = [True, False, True]
    state.agents_arrived = [False, False, True]
    state.gems_collected = [False, True]
    state.sources_enabled = [True, False]
    world.set_state(state)
    flags = [0, 1, 1, 0]
    assert_vector(
        world.state_vector(),
        [
            *block(0.1, 0.125, 0),
            *block(0.5, 0.125, 1, alive=False),
            *block(4.5 / W, 0.625, 2, arrived=True),
            *flags,
        ],
    )
    assert_vector(
        world.observation(0),
        [
            *block(0.1, 0.125, 0),
            *block(0.4, 0.0, 1, alive=False),
            *block(4.0 / W, 2.0 / H, 2, arrived=True),  # 0.8, 0.5
            *flags,
        ],
    )


def test_velocities_are_normalized_and_never_relative():
    # Acceleration model, one tick from rest: v = damping * a * max_acceleration * dt
    # = 0.9 * 8 * 0.05 = 0.36, i.e. 0.18 * max_speed, and the agent moves by 0.36 * 0.05 = 0.018.
    world = ContinuousWorld(MAP, PhysicsParams(action_model="acceleration"))
    world.step([(1.0, 0.0), (0.0, 1.0), (0.0, 0.0)], 1)
    a0 = block(0.518 / W, 0.125, 0, vx=0.18)
    a1 = block(0.5, 0.518 / H, 1, vy=0.18)
    assert_vector(world.state_vector(), [*a0, *a1, *block(0.1, 0.625, 2), 0, 0, 1, 1])
    assert_vector(
        world.observation(2),
        [
            *block(0.1, 0.625, 2),
            *block(0.018 / W, -2.0 / H, 0, vx=0.18),  # relative x and y, absolute velocity
            *block(2.0 / W, (0.518 - 2.5) / H, 1, vy=0.18),
            *[0, 0, 1, 1],
        ],
    )


@pytest.mark.parametrize("action_model", ["velocity", "acceleration"])
def test_velocity_features_are_the_state_velocities(action_model: str):
    """With the velocity model, the reported velocity is whatever the state holds (the contract)."""
    params = PhysicsParams(action_model=action_model)
    world = ContinuousWorld(MAP, params)
    world.step([(0.6, 0.3), (0.0, 1.0), (0.5, -0.5)], 4)
    vector = world.state_vector()
    velocities = np.array(world.get_state().velocities, dtype=np.float64) / params.max_speed
    for agent in range(3):
        start = agent * 9
        np.testing.assert_allclose(vector[start + 2 : start + 4], velocities[agent], rtol=1e-6, atol=1e-7)


def test_observation_and_state_vector_change_after_a_step():
    world = ContinuousWorld(MAP)
    before = [world.observation(i) for i in range(3)]
    world.step([(1.0, 0.0), (0.0, 0.0), (0.0, 0.0)], 2)
    # Agent 0 moved by 0.2: its absolute x in its own observation, and its relative x in the others.
    np.testing.assert_allclose(world.observation(0)[0] - before[0][0], 0.2 / W, rtol=1e-5)
    np.testing.assert_allclose(world.observation(1)[9] - before[1][9], 0.2 / W, rtol=1e-5)
    np.testing.assert_allclose(world.observation(2)[9] - before[2][9], 0.2 / W, rtol=1e-5)
    np.testing.assert_allclose(world.state_vector()[0], 0.7 / W, rtol=1e-6)
