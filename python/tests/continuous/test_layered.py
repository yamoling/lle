"""Tests of the `LAYERED` observation of `ContinuousWorld` (round B, §2.1).

`world.layered(res)` has the shape `(C, height * res, width * res)` with `C = 2 * n_colours + 4`:
the agent channels by colour, the laser channels by colour, then `WALL`, `VOID`, `GEM` and `EXIT`.
A pixel is a `1/res` square sub-cell, set when its centre lies in the shape it encodes. At `res`
sub-cells per cell, the pixel centres of a cell are at the offsets `(k + 0.5) / res`:
- `res = 1`: `0.5`;
- `res = 3`: `1/6`, `1/2`, `5/6`;
- `res = 4`: `0.125`, `0.375`, `0.625`, `0.875`.

With the default physics, a disc of radius `0.3` centred in a cell covers the central pixel at
`res = 3` (the next ones are `1/3` away) and the central 2 × 2 pixels at `res = 4` (`0.177` away,
the next ones `0.395`); so does a gem of radius `0.25`. A beam of width `0.1` only lights the pixels
whose centre is within `0.05` of its centre line, i.e. the centre row (or column) at odd `res`.
"""

import lle
import numpy as np
import pytest
from lle import WorldState
from lle.continuous import ContinuousWorld, PhysicsParams
from lle.observations import Layered

from .common import SAME_COLOUR

MAP = """
S0  . G . X
L1E . . . .
S1  . V . X
"""
# Width 5, height 3, n_colours = 2, hence 8 channels. Agent 0 (colour 0) on (0, 0), agent 1
# (colour 1) on (2, 0). L1E on (1, 0): a beam of colour 1 along y = 1.5, from x = 1 to the grid edge
# x = 5 (cells (1, 1) to (1, 4)). A gem on (0, 2), a void on (2, 2), exits on (0, 4) and (2, 4).
A0, A1, L0, L1, WALL, VOID, GEM, EXIT = range(8)


def lle_layered(world: lle.World) -> np.ndarray:
    """
    LLE's `Layered` observation of `world` (identical for every agent), without its layer axis.

    @ai-generated
    """
    return Layered(world).observe()[0][..., 0]


def block(res: int, i: int, j: int) -> tuple[slice, slice]:
    """The pixels of the cell `(i, j)` at the resolution `res`."""
    return slice(i * res, (i + 1) * res), slice(j * res, (j + 1) * res)


def expected_res_3() -> np.ndarray:
    """
    The layered observation of `MAP` at `res = 3` after `reset`, derived by hand.

    @ai-generated
    """
    obs = np.zeros((8, 9, 15), dtype=np.float32)
    obs[A0, 1, 1] = 1.0  # the centre pixel of (0, 0)
    obs[A1, 7, 1] = 1.0  # the centre pixel of (2, 0)
    obs[(L1, *block(3, 1, 0))] = -1.0  # the source
    obs[L1, 4, 3:15] = 1.0  # the centre row of the beam, from x = 1 + 1/6 to x = 5 - 1/6
    obs[(WALL, *block(3, 1, 0))] = 1.0  # sources are walls, as in LLE's `wall_pos`
    obs[(VOID, *block(3, 2, 2))] = 1.0
    obs[GEM, 1, 7] = 1.0  # the centre pixel of (0, 2)
    obs[(EXIT, *block(3, 0, 4))] = 1.0
    obs[(EXIT, *block(3, 2, 4))] = 1.0
    return obs


@pytest.mark.parametrize("level", range(1, 7))
def test_layered_at_res_1_equals_lle_on_every_level(level: int):
    world = lle.World.level(level)
    world.reset()
    expected = lle_layered(world)
    layered = ContinuousWorld.level(level).layered(1)
    assert layered.dtype == np.float32
    np.testing.assert_array_equal(layered, expected)


def test_layered_at_res_1_equals_lle_with_a_cut_beam_and_a_collected_gem():
    # Agent 1 stands on (1, 2), in the beam of its own colour. In LLE, the beam is off from its cell
    # on. In CLLE, the beam is cut at s = 1.5 - r = 1.2 from the source edge (x = 2.2): the pixel
    # centre of (1, 1) (s = 0.5) is lit, those of (1, 2) and beyond are not.
    world = lle.World(MAP)
    world.reset()
    world.set_state(WorldState([(0, 0), (1, 2)], [True]))
    cworld = ContinuousWorld(MAP)
    state = cworld.get_state()
    state.positions = [(0.5, 0.5), (2.5, 1.5)]
    state.gems_collected = [True]
    cworld.set_state(state)
    expected = lle_layered(world)
    assert expected[L1, 1].tolist() == [-1, 1, 0, 0, 0]
    np.testing.assert_array_equal(cworld.layered(1), expected)


def test_layered_at_res_3_matches_the_hand_derived_rasterization():
    world = ContinuousWorld(MAP)
    np.testing.assert_array_equal(world.layered(3), expected_res_3())


def test_layered_at_res_4_rasterizes_off_centre_discs_and_gems():
    # Agent 0 is moved to the corner (2.0, 1.0) of the cells (0, 1), (0, 2), (1, 1) and (1, 2). The
    # pixel centres within 0.3 of it are (1.875 or 2.125, 0.875 or 1.125), one in each cell: rows
    # 3–4, columns 7–8. The gem of (0, 2) covers the central 2 × 2 pixels of its cell: rows 1–2,
    # columns 9–10, and agent 1 those of (2, 0): rows 9–10, columns 1–2.
    world = ContinuousWorld(MAP)
    state = world.get_state()
    state.positions = [(2.0, 1.0), (0.5, 2.5)]
    world.set_state(state)
    layered = world.layered(4)
    assert layered.shape == (8, 12, 20)
    expected_a0 = np.zeros((12, 20), dtype=np.float32)
    expected_a0[3:5, 7:9] = 1.0
    np.testing.assert_array_equal(layered[A0], expected_a0)
    expected_a1 = np.zeros((12, 20), dtype=np.float32)
    expected_a1[9:11, 1:3] = 1.0
    np.testing.assert_array_equal(layered[A1], expected_a1)
    expected_gem = np.zeros((12, 20), dtype=np.float32)
    expected_gem[1:3, 9:11] = 1.0
    np.testing.assert_array_equal(layered[GEM], expected_gem)
    # Cells are filled entirely.
    expected_void = np.zeros((12, 20), dtype=np.float32)
    expected_void[8:12, 8:12] = 1.0
    np.testing.assert_array_equal(layered[VOID], expected_void)
    expected_exit = np.zeros((12, 20), dtype=np.float32)
    expected_exit[0:4, 16:20] = 1.0
    expected_exit[8:12, 16:20] = 1.0
    np.testing.assert_array_equal(layered[EXIT], expected_exit)
    expected_wall = np.zeros((12, 20), dtype=np.float32)
    expected_wall[4:8, 0:4] = 1.0
    np.testing.assert_array_equal(layered[WALL], expected_wall)
    # Every pixel of the source is -1, and nothing has colour 0.
    np.testing.assert_array_equal(layered[(L1, *block(4, 1, 0))], -1.0)
    np.testing.assert_array_equal(layered[L0], 0.0)


def test_layered_at_res_4_rasterizes_a_wide_beam_and_its_cut():
    # beam_width = 0.3: the pixel rows of the beam row 1 have their centres at y = 1.125, 1.375,
    # 1.625 and 1.875, i.e. 0.375, 0.125, 0.125 and 0.375 from the centre line: rows 5 and 6 are lit
    # (0.125 < 0.15), rows 4 and 7 are not. The pixel column 4 + k has its centre at x = 1.125 + k/4.
    world = ContinuousWorld(MAP, PhysicsParams(beam_width=0.3))
    layered = world.layered(4)
    np.testing.assert_array_equal(layered[L1, 5:7, 4:20], 1.0)
    np.testing.assert_array_equal(layered[L1, 4, 4:20], 0.0)
    np.testing.assert_array_equal(layered[L1, 7, 4:20], 0.0)
    np.testing.assert_array_equal(layered[L1, 4:8, 0:3], -1.0)
    # Agent 1 moves into the beam at (2.5, 1.5): the beam is cut at x = 2.2. The column 8 (x = 2.125)
    # is still lit, the column 9 (x = 2.375, 0.175 > 0.15 past the cut) and beyond are not.
    state = world.get_state()
    state.positions = [(0.5, 0.5), (2.5, 1.5)]
    world.set_state(state)
    layered = world.layered(4)
    np.testing.assert_array_equal(layered[L1, 5:7, 4:9], 1.0)
    np.testing.assert_array_equal(layered[L1, 4:8, 9:20], 0.0)


def test_dead_agents_are_not_drawn_and_do_not_cut_beams():
    # Agent 1 lies dead in its own beam: the beam is not cut, and agent 1 disappears.
    world = ContinuousWorld(MAP)
    state = world.get_state()
    state.positions = [(0.5, 0.5), (2.5, 1.5)]
    state.agents_alive = [True, False]
    world.set_state(state)
    layered = world.layered(1)
    np.testing.assert_array_equal(layered[A1], 0.0)
    assert layered[L1, 1].tolist() == [-1, 1, 1, 1, 1]
    assert layered[A0, 0, 0] == 1.0


def test_arrived_agents_are_drawn():
    world = ContinuousWorld(MAP)
    state = world.get_state()
    state.positions = [(4.5, 0.5), (0.5, 2.5)]
    state.agents_arrived = [True, False]
    world.set_state(state)
    layered = world.layered(1)
    expected = np.zeros((3, 5), dtype=np.float32)
    expected[0, 4] = 1.0
    np.testing.assert_array_equal(layered[A0], expected)


def test_disabled_source_has_no_beam_but_is_still_encoded():
    world = ContinuousWorld(MAP)
    state = world.get_state()
    state.sources_enabled = [False]
    world.set_state(state)
    layered = world.layered(1)
    assert layered[L1, 1].tolist() == [-1, 0, 0, 0, 0]
    assert layered[WALL, 1, 0] == 1.0


def test_collected_gem_disappears():
    world = ContinuousWorld(MAP)
    state = world.get_state()
    state.gems_collected = [True]
    world.set_state(state)
    np.testing.assert_array_equal(world.layered(4)[GEM], 0.0)


@pytest.mark.parametrize("res", [1, 2, 3, 4, 7])
def test_layered_shape_matches_the_array(res: int):
    world = ContinuousWorld(MAP)
    assert tuple(world.layered_shape(res)) == (8, 3 * res, 5 * res)
    assert world.layered(res).shape == (8, 3 * res, 5 * res)


def test_layered_default_resolution_is_4():
    world = ContinuousWorld(MAP)
    np.testing.assert_array_equal(world.layered(), world.layered(4))


def test_agents_sharing_a_colour_share_their_channel():
    # Two agents of colour 0 and one source of colour 0: n_colours = 1, hence 2 + 4 channels. Unlike
    # LLE's `Layered`, which refuses such worlds, both agents are drawn in the channel of colour 0.
    world = ContinuousWorld(SAME_COLOUR)
    assert world.n_colours == 1
    layered = world.layered(1)
    assert layered.shape == (6, 3, 4)
    expected = np.zeros((3, 4), dtype=np.float32)
    expected[0, 0] = expected[2, 1] = 1.0
    np.testing.assert_array_equal(layered[0], expected)


@pytest.mark.parametrize("res", [0, -1])
def test_invalid_resolution_is_a_value_error(res: int):
    world = ContinuousWorld(MAP)
    with pytest.raises(ValueError):
        world.layered(res)
    with pytest.raises(ValueError):
        world.layered_shape(res)
