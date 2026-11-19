"""Tests of the `LIDAR` observation of `ContinuousWorld` (round B, §2.2).

`world.lidar(agent, n_rays, range)` casts `n_rays` rays from the centre of `agent`, evenly spaced
over 360°: ray 0 points east (+x), and the next ones turn counter-clockwise on screen, so towards
the north (-y) first. Ray `k` has the direction `(cos(2πk/n), -sin(2πk/n))`. For each ray and each
category, the value is the distance to the first hit divided by `range`, or `1.0` without a hit
within `range`. The categories are:
0. wall or laser source (cells), and the grid boundary; 1. void (cells); 2. other alive agents (discs of radius 0.3);
3. uncollected gems (circles of radius 0.25); 4. exits (cells); then 5 + c. the beams of colour `c`
after their cuts, as segments of width `beam_width` (0.1 by default).

The layout is ray-major (`[ray 0: cat 0..N, ray 1: ...]`), followed by the observer's own STATE
block (`6 + n_colours` values, as at the start of `world.observation(agent)`).

The grid boundary is solid for the agents, so the rays hit it in the wall category, from inside
the grid. A hit at exactly `range` is clipped to `1.0`, like no hit.
"""

import math

import numpy as np
import pytest
from lle.continuous import ContinuousWorld

from .common import NORTH, STILL

WALLS = """
.  .  .  @  .  .
@  .  .  S0 .  @
.  .  .  .  .  .
.  .  .  .  .  .
X  .  .  .  .  .
.  .  .  @  .  .
"""
# Width 6, height 6, n_colours = 1: 6 categories and an own block of 7 values. The agent is at
# (3.5, 1.5), and the nearest wall is at a different distance in each axis direction:
# - east: the wall (1, 5), whose left edge is x = 5: 1.5;
# - north: the wall (0, 3), whose bottom edge is y = 1: 0.5;
# - west: the wall (1, 0), whose right edge is x = 1: 2.5;
# - south: the wall (5, 3), whose top edge is y = 5: 3.5.
# None of these rays crosses the exit (4, 0).

MIXED = """
.   .  .  .  @  .  .  .  .
L1E .  .  .  .  .  .  .  .
.   .  .  .  .  .  .  .  .
@   S1 .  .  S0 .  .  G  @
.   .  .  .  .  .  .  .  .
.   .  .  .  V  .  .  .  .
.   .  .  .  X  .  .  .  X
"""
# Width 9, height 7, n_colours = 2: 7 categories and an own block of 8 values. Agent 0 (colour 0) is
# at (4.5, 3.5), agent 1 (colour 1) at (1.5, 3.5). The beam of colour 1 runs along y = 1.5 from
# x = 1 to the grid edge, and its lower edge is at y = 1.55.
# From agent 0:
# - east: the gem (3, 7), centred at x = 7.5: 3 - 0.25 = 2.75; the wall (3, 8) at x = 8: 3.5;
# - north: the beam at y = 1.55: 1.95; the wall (0, 4), whose bottom edge is y = 1: 2.5;
# - west: agent 1, centred at x = 1.5: 3 - 0.3 = 2.7; the wall (3, 0) at x = 1: 3.5;
# - south: the void (5, 4) at y = 5: 1.5; the exit (6, 4) at y = 6: 2.5; the grid edge y = 7: 3.5.
# From agent 1:
# - east: agent 0 at 2.7; the gem at 6 - 0.25 = 5.75 and the wall (3, 8) at 6.5, out of range;
# - north: its own beam at 1.95, then the grid edge y = 0: 3.5;
# - west: the wall (3, 0) at x = 1: 0.5;
# - south: nothing before the grid edge y = 7: 3.5.
WALL, VOID, AGENT, GEM, EXIT, BEAM_0, BEAM_1 = range(7)

DIAGONAL = """
.  .  .  G
.  .  .  .
.  .  .  .
S0 .  .  X
"""
# Width 4, height 4. The agent is at (0.5, 3.5), and the gem centred at (3.5, 0.5) lies exactly
# north-east of it, at 3·sqrt(2) - 0.25 from its centre. With 8 rays, north-east is ray 1. The exit
# (3, 3) is only crossed by ray 0 (east), at x = 3: 2.5.


def rays(lidar: np.ndarray, n_rays: int, n_categories: int) -> np.ndarray:
    """The ray-major part of a lidar vector, as an `(n_rays, n_categories)` array."""
    return lidar[: n_rays * n_categories].reshape(n_rays, n_categories)


def assert_rays(actual: np.ndarray, expected: list[list[float]]):
    """
    Check every entry of the `(n_rays, n_categories)` array `actual` against `expected`. The
    distances are computed in `f64` and exported as `f32`.

    @ai-generated
    """
    expected_array = np.array(expected, dtype=np.float64)
    assert actual.shape == expected_array.shape
    np.testing.assert_allclose(actual, expected_array, rtol=1e-6, atol=1e-6)


def test_lidar_size_and_dtype():
    world = ContinuousWorld(MIXED)
    for n_rays in (1, 3, 4, 16):
        # n_rays · (5 + n_colours) + (6 + n_colours)
        assert world.lidar_size(n_rays) == n_rays * 7 + 8
        lidar = world.lidar(0, n_rays=n_rays)
        assert lidar.dtype == np.float32
        assert lidar.shape == (n_rays * 7 + 8,)
    assert world.lidar(1).shape == (world.lidar_size(16),)


def test_rays_start_east_and_turn_counter_clockwise_on_screen():
    # East, north, west, south: 1.5, 0.5, 2.5 and 3.5, divided by the range 5.
    world = ContinuousWorld(WALLS)
    lidar = world.lidar(0, n_rays=4, range=5.0)
    assert_rays(
        rays(lidar, 4, 6),
        [
            [0.3, 1, 1, 1, 1, 1],
            [0.1, 1, 1, 1, 1, 1],
            [0.5, 1, 1, 1, 1, 1],
            [0.7, 1, 1, 1, 1, 1],
        ],
    )


def test_a_diagonal_ray_turns_towards_the_north_first():
    # Ray 1 of 8 is north-east, not south-east: it alone hits the gem.
    world = ContinuousWorld(DIAGONAL)
    r = rays(world.lidar(0, n_rays=8, range=5.0), 8, 6)
    gem_distance = (3 * math.sqrt(2) - 0.25) / 5
    np.testing.assert_allclose(r[:, GEM], [1, gem_distance, 1, 1, 1, 1, 1, 1], rtol=1e-6)
    np.testing.assert_allclose(r[:, EXIT], [0.5, 1, 1, 1, 1, 1, 1, 1], rtol=1e-6)


def test_each_category_is_hit_at_its_own_distance():
    # From agent 0: a wall, a gem, a beam of another colour, an agent of another colour, a void and
    # an exit. The categories do not occlude one another: the walls behind the gem and behind
    # agent 1 are still seen.
    world = ContinuousWorld(MIXED)
    assert_rays(
        rays(world.lidar(0, n_rays=4, range=5.0), 4, 7),
        [
            [0.7, 1, 1, 0.55, 1, 1, 1],  # east: wall 3.5, gem 2.75
            [0.5, 1, 1, 1, 1, 1, 0.39],  # north: wall 2.5, beam of colour 1 at 1.95
            [0.7, 1, 0.54, 1, 1, 1, 1],  # west: wall 3.5, agent 1 at 2.7
            [0.7, 0.3, 1, 1, 0.5, 1, 1],  # south: grid edge 3.5, void 1.5, exit 2.5
        ],
    )


def test_hits_beyond_the_range_read_one():
    # From agent 1, the gem (5.75) and the wall (6.5) to the east are out of range, agent 0 (2.7)
    # is not. Its own beam is seen like any other beam.
    world = ContinuousWorld(MIXED)
    assert_rays(
        rays(world.lidar(1, n_rays=4, range=5.0), 4, 7),
        [
            [1, 1, 0.54, 1, 1, 1, 1],
            [0.7, 1, 1, 1, 1, 1, 0.39],
            [0.1, 1, 1, 1, 1, 1, 1],
            [0.7, 1, 1, 1, 1, 1, 1],
        ],
    )


def test_distances_are_divided_by_the_range():
    # With a range of 2: the beam at 1.95 and the void at 1.5 remain, everything else is too far.
    world = ContinuousWorld(MIXED)
    assert_rays(
        rays(world.lidar(0, n_rays=4, range=2.0), 4, 7),
        [
            [1, 1, 1, 1, 1, 1, 1],
            [1, 1, 1, 1, 1, 1, 0.975],
            [1, 1, 1, 1, 1, 1, 1],
            [1, 0.75, 1, 1, 1, 1, 1],
        ],
    )


def test_rays_see_the_beams_after_their_cuts():
    # Agent 1 stands in its own beam at (3.5, 1.5): the beam is cut at x = 3.5 - 0.3 = 3.2, so the
    # north ray of agent 0 (x = 4.5) no longer crosses it.
    world = ContinuousWorld(MIXED)
    state = world.get_state()
    state.positions = [(4.5, 3.5), (3.5, 1.5)]
    world.set_state(state)
    assert rays(world.lidar(0, n_rays=4), 4, 7)[1, BEAM_1] == 1.0


def test_a_disabled_source_has_no_beam():
    world = ContinuousWorld(MIXED)
    state = world.get_state()
    state.sources_enabled = [False]
    world.set_state(state)
    np.testing.assert_array_equal(rays(world.lidar(0, n_rays=4), 4, 7)[:, BEAM_1], 1.0)


def test_dead_agents_are_not_seen_and_dead_observers_still_cast_rays():
    world = ContinuousWorld(MIXED)
    alive = world.lidar(0, n_rays=4)
    state = world.get_state()
    state.agents_alive = [False, True]
    world.set_state(state)
    # Agent 1 no longer sees agent 0 to the east.
    assert rays(world.lidar(1, n_rays=4), 4, 7)[0, AGENT] == 1.0
    # Agent 0 casts the same rays from where it lies, and its own block says it is dead.
    dead = world.lidar(0, n_rays=4)
    np.testing.assert_array_equal(dead[:28], alive[:28])
    assert dead[28 + 4] == 0.0  # alive
    assert dead[28 + 5] == 0.0  # arrived


def test_an_arrived_observer_inside_an_exit_reads_zero_for_exits():
    # The agent stands on the exit (3, 3), centred at (3.5, 3.5): every ray starts inside the exit
    # cell, so the first hit is at distance 0. The gem (3, 3 cells to the north, i.e. ray 2 of 8) is
    # at 3 - 0.25 = 2.75.
    world = ContinuousWorld(DIAGONAL)
    state = world.get_state()
    state.positions = [(3.5, 3.5)]
    state.agents_arrived = [True]
    world.set_state(state)
    lidar = world.lidar(0, n_rays=8, range=5.0)
    r = rays(lidar, 8, 6)
    np.testing.assert_array_equal(r[:, EXIT], 0.0)
    np.testing.assert_allclose(r[:, GEM], [1, 1, 0.55, 1, 1, 1, 1, 1], rtol=1e-6)
    assert lidar[48 + 5] == 1.0  # arrived


def test_the_own_state_block_is_appended():
    # After a step, the block also carries the velocity of the observer.
    world = ContinuousWorld(MIXED)
    world.step([NORTH, STILL], 3)
    for agent in range(2):
        lidar = world.lidar(agent, n_rays=4)
        own_block = world.observation(agent)[:8]
        np.testing.assert_array_equal(lidar[28:], own_block)
    np.testing.assert_allclose(world.lidar(0, n_rays=4)[28:32], [4.5 / 9, (3.5 - 0.3) / 7, 0.0, -1.0], rtol=1e-6)


def test_lidar_default_configuration_is_16_rays_of_range_5():
    world = ContinuousWorld(MIXED)
    np.testing.assert_array_equal(world.lidar(0), world.lidar(0, n_rays=16, range=5.0))


@pytest.mark.parametrize(
    "kwargs",
    [
        {"n_rays": 0},
        {"n_rays": -1},
        {"range": 0.0},
        {"range": -1.0},
        {"range": math.inf},
        {"range": math.nan},
    ],
    ids=["no-ray", "negative-rays", "zero-range", "negative-range", "infinite-range", "nan-range"],
)
def test_invalid_lidar_arguments_are_value_errors(kwargs: dict):
    world = ContinuousWorld(MIXED)
    with pytest.raises(ValueError):
        world.lidar(0, **kwargs)


def test_invalid_lidar_sizes_are_value_errors():
    world = ContinuousWorld(MIXED)
    with pytest.raises(ValueError):
        world.lidar_size(0)


@pytest.mark.parametrize("agent", [2, -1, 2**64])
def test_unknown_agent_is_an_index_error(agent: int):
    world = ContinuousWorld(MIXED)
    with pytest.raises(IndexError):
        world.lidar(agent)


def test_the_grid_boundary_is_hit_as_a_wall():
    # No wall at all: from (0.5, 0.5) in the 3 × 2 grid, the boundary is 2.5 away to the east, 0.5
    # to the north and to the west, and 1.5 to the south. The diagonal rays leave through the top
    # or left edge, 0.5·sqrt(2) away, except the south-east one, through the bottom edge.
    world = ContinuousWorld("S0 . X\n. . .")
    r = rays(world.lidar(0, n_rays=8, range=5.0), 8, 6)
    s2 = math.sqrt(2)
    expected = [2.5, 0.5 * s2, 0.5, 0.5 * s2, 0.5, 0.5 * s2, 1.5, 1.5 * s2]
    np.testing.assert_allclose(r[:, WALL], np.array(expected) / 5, rtol=1e-6)
    # Nothing else is hit, except the exit (0, 2) to the east, 1.5 away.
    np.testing.assert_allclose(r[0, 1:], [1, 1, 1, 0.3, 1], rtol=1e-6)


def test_a_hit_at_exactly_the_range_reads_one():
    # The void (5, 4) starts exactly 1.5 south of agent 0: with a range of 1.5, the value is clipped
    # to 1, like no hit; with a slightly longer range, it is just below 1.
    world = ContinuousWorld(MIXED)
    assert rays(world.lidar(0, n_rays=4, range=1.5), 4, 7)[3, VOID] == 1.0
    assert rays(world.lidar(0, n_rays=4, range=1.5 + 1e-6), 4, 7)[3, VOID] < 1.0


def test_a_blocker_sees_its_own_beam_end_at_its_disc():
    # Agent 1 (colour 1) stands in its own beam at (3.5, 1.5): the segment ends at its disc, 0.3 to
    # the west (towards the source). To the east, the beam is cut: nothing.
    world = ContinuousWorld(MIXED)
    state = world.get_state()
    state.positions = [(4.5, 3.5), (3.5, 1.5)]
    world.set_state(state)
    r = rays(world.lidar(1, n_rays=4, range=5.0), 4, 7)
    assert r[2, BEAM_1] == pytest.approx(0.3 / 5)
    assert r[0, BEAM_1] == 1.0
