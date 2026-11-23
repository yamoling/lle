"""Tests of the `LAYERED` observation of `ContinuousWorld` (round B, §2.1).

`world.layered(res)` has the shape `(C, height * res, width * res)` with `C = 2 * n_colours + 4`:
the agent channels by colour, the laser channels by colour, then `WALL`, `VOID`, `GEM` and `EXIT`.
A pixel is a `1/res` square sub-cell, set when its square overlaps the shape it encodes with a
positive area (for a disc: the point of the square closest to the centre is strictly within the
radius). At `res` sub-cells per cell, the pixels of a cell span `[k / res, (k + 1) / res]`.

With the default physics, a disc of radius `0.3` centred in a cell:
- at `res = 1`, only overlaps its cell (the next cells are `0.5` away);
- at `res = 2`, overlaps the 4 pixels of its cell, which all touch the centre;
- at `res = 3`, overlaps the 9 pixels of its cell (the corner pixels are `sqrt(2)/6 ≈ 0.236` away);
- at `res = 4`, overlaps the 12 pixels of its cell that are not corners (`0.25` away for the edge
  pixels, `0.354` for the corners).

A gem of radius `0.25` behaves the same, except at `res = 4`: the edge pixels are exactly `0.25`
away, not strictly within the radius, so it only overlaps the central 2 × 2 pixels. A beam (`0.1`
wide, from the source edge to the cut) lights the centre row (or column) at odd `res` and the two
rows on either side of its centre line at even `res`.
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
    LLE's `Layered` observation of `world` (identical for every agent): `(C, H, W)`, since the map has one layer.

    @ai-generated
    """
    return Layered(world).observe()[0]


def block(res: int, i: int, j: int) -> tuple[slice, slice]:
    """The pixels of the cell `(i, j)` at the resolution `res`."""
    return slice(i * res, (i + 1) * res), slice(j * res, (j + 1) * res)


def expected_res_3() -> np.ndarray:
    """
    The layered observation of `MAP` at `res = 3` after `reset`, derived by hand.

    @ai-generated
    """
    obs = np.zeros((8, 9, 15), dtype=np.float32)
    obs[(A0, *block(3, 0, 0))] = 1.0  # the 9 pixels of (0, 0)
    obs[(A1, *block(3, 2, 0))] = 1.0  # the 9 pixels of (2, 0)
    obs[(L1, *block(3, 1, 0))] = -1.0  # the source
    obs[L1, 4, 3:15] = 1.0  # the centre row of the beam, from x = 1 + 1/6 to x = 5 - 1/6
    obs[(WALL, *block(3, 1, 0))] = 1.0  # sources are walls, as in LLE's `wall_pos`
    obs[(VOID, *block(3, 2, 2))] = 1.0
    obs[(GEM, *block(3, 0, 2))] = 1.0  # the 9 pixels of (0, 2): the corners are 0.236 < 0.25 away
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


def test_layered_at_res_1_differs_from_lle_only_in_the_cell_of_a_beam_blocker():
    # Agent 1 stands on (1, 2), in the beam of its own colour. In LLE, the beam is off from its cell
    # on. In CLLE, the beam is cut at s = 1.5 - r = 1.2 from the source edge (x = 2.2): it overlaps
    # the cells (1, 1) and (1, 2), so the cell of the blocker is lit in CLLE and not in LLE. This is
    # the only (documented) difference; the gem, collected in both, is absent in both.
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
    layered = cworld.layered(1)
    assert layered[L1, 1].tolist() == [-1, 1, 1, 0, 0]
    expected[L1, 1, 2] = 1.0
    np.testing.assert_array_equal(layered, expected)


def test_layered_at_res_3_matches_the_hand_derived_rasterization():
    world = ContinuousWorld(MAP)
    np.testing.assert_array_equal(world.layered(3), expected_res_3())


def test_layered_at_res_4_rasterizes_off_centre_discs_and_gems():
    # Agent 0 is moved to the corner (2.0, 1.0) of the cells (0, 1), (0, 2), (1, 1) and (1, 2). The
    # pixels that touch it (rows 3–4, columns 7–8) overlap its disc, and so do the pixels 0.25 away
    # along one axis only (rows 3–4 with columns 6 and 9, rows 2 and 5 with columns 7–8). The gem
    # of (0, 2) only overlaps the central 2 × 2 pixels of its cell: rows 1–2, columns 9–10. Agent 1
    # overlaps the 12 pixels of (2, 0) that are not corners: rows 8–11, columns 0–3.
    world = ContinuousWorld(MAP)
    state = world.get_state()
    state.positions = [(2.0, 1.0), (0.5, 2.5)]
    world.set_state(state)
    layered = world.layered(4)
    assert layered.shape == (8, 12, 20)
    expected_a0 = np.zeros((12, 20), dtype=np.float32)
    expected_a0[3:5, 6:10] = 1.0
    expected_a0[[2, 5], 7:9] = 1.0
    np.testing.assert_array_equal(layered[A0], expected_a0)
    expected_a1 = np.zeros((12, 20), dtype=np.float32)
    expected_a1[8:12, 0:4] = 1.0
    expected_a1[[8, 8, 11, 11], [0, 3, 0, 3]] = 0.0
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
    # beam_width = 0.3: the beam spans y in [1.35, 1.65]. The pixel rows 5 ([1.25, 1.5]) and 6
    # ([1.5, 1.75]) overlap it, the rows 4 ([1, 1.25]) and 7 ([1.75, 2]) do not. The pixel column
    # 4 + k spans x in [1 + k/4, 1.25 + k/4].
    world = ContinuousWorld(MAP, PhysicsParams(beam_width=0.3))
    layered = world.layered(4)
    np.testing.assert_array_equal(layered[L1, 5:7, 4:20], 1.0)
    np.testing.assert_array_equal(layered[L1, 4, 4:20], 0.0)
    np.testing.assert_array_equal(layered[L1, 7, 4:20], 0.0)
    np.testing.assert_array_equal(layered[L1, 4:8, 0:3], -1.0)
    # Agent 1 moves into the beam at (2.5, 1.5): the beam is cut at x = 2.2. The column 8
    # ([2, 2.25]) still overlaps it, the column 9 ([2.25, 2.5]) and beyond do not.
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
    assert world.layered_shape() == world.layered_shape(4)
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
