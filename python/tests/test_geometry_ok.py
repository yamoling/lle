"""Unit tests for generator._geometry.geometry_ok."""

from __future__ import annotations

from lle.generator.candidates import CandidateLayout
from lle.tiles import Direction

# Helpers to build minimal layouts
_AGENTS = [(0, 0)]
_EXITS = [(4, 4)]


def _layout(lasers, *, walls=(), agents=_AGENTS, exits=_EXITS):
    return CandidateLayout(5, 5, agents=list(agents), exits=list(exits), walls=list(walls), lasers=list(lasers))


# ---------------------------------------------------------------------------
# Valid layout
# ---------------------------------------------------------------------------


def test_no_lasers_is_valid():
    layout = _layout(lasers=[])
    assert layout.is_geometry_valid()


def test_valid_laser_south():
    # Source at (0, 2) firing SOUTH on a 5x5 grid → beam covers (1,2),(2,2),(3,2),(4,2)
    assert _layout(lasers=[(0, (0, 2), Direction.SOUTH)]).is_geometry_valid()


def test_valid_laser_east():
    assert _layout(lasers=[(0, (2, 0), Direction.EAST)]).is_geometry_valid()


# ---------------------------------------------------------------------------
# Laser pointing immediately out of the grid
# ---------------------------------------------------------------------------
def test_reject_laser_pointing_north_from_top_row():
    # Source on row 0 firing NORTH → first step is out of bounds immediately
    assert not _layout(lasers=[(0, (0, 2), Direction.NORTH)]).is_geometry_valid()


def test_reject_laser_pointing_south_from_bottom_row():
    assert not _layout(lasers=[(0, (4, 2), Direction.SOUTH)]).is_geometry_valid()


def test_reject_laser_pointing_west_from_left_col():
    assert not _layout(lasers=[(0, (2, 0), Direction.WEST)]).is_geometry_valid()


def test_reject_laser_pointing_east_from_right_col():
    assert not _layout(lasers=[(0, (2, 4), Direction.EAST)]).is_geometry_valid()


# ---------------------------------------------------------------------------
# Laser beam shorter than 2 tiles
# ---------------------------------------------------------------------------
def test_reject_laser_blocked_after_one_tile_by_wall():
    # Source at (2, 0) EAST; wall at (2, 1) → beam = [(2,1)] but stopped by wall before appending,
    # actually beam_tiles stops *before* the wall, so (2,1) is the wall → beam = [] if wall is at (2,1)
    # Let's be precise: beam_tiles stops when it hits the wall cell, so (2,1) is NOT included.
    # With wall at (2,2): beam = [(2,1)] → length 1 → rejected.
    assert not _layout(lasers=[(0, (2, 0), Direction.EAST)], walls=[(2, 2)]).is_geometry_valid()


def test_reject_laser_blocked_immediately_by_wall():
    # Wall directly in front of source → beam is empty (length 0) → rejected.
    assert not _layout(lasers=[(0, (2, 0), Direction.EAST)], walls=[(2, 1)]).is_geometry_valid()


def test_reject_laser_one_tile_from_grid_corner():
    # Source at (0, 3) firing EAST on a 5-col grid → beam = [(0,4)] (length 1) → rejected.
    assert not _layout(lasers=[(0, (0, 3), Direction.EAST)]).is_geometry_valid()


def test_reject_laser_one_tile_from_another_laser():
    # Two lasers where the second's beam is immediately blocked by the first's source.
    # Laser A at (2, 0) EAST; Laser B at (2, 2) EAST.
    # Beam of B starts at (2,3),(2,4) → length 2 → fine.
    # But if B is at (2, 3) EAST → beam = [(2,4)] → length 1 → rejected.
    layout = _layout(
        lasers=[
            (0, (2, 0), Direction.EAST),
            (1, (2, 3), Direction.EAST),
        ]
    )
    assert not layout.is_geometry_valid()


# ---------------------------------------------------------------------------
# Exit on beam
# ---------------------------------------------------------------------------
def test_reject_exit_on_laser_beam():
    # Source at (0, 2) SOUTH; exit at (2, 2) which is on the beam → rejected.
    assert not _layout(lasers=[(0, (0, 2), Direction.SOUTH)], exits=[(2, 2)]).is_geometry_valid()


def test_accept_exit_behind_wall_shielding_beam():
    # Source at (0, 2) SOUTH; wall at (1, 2) blocks the beam; exit at (2, 2).
    # Beam is empty (wall at (1,2) stops it immediately) → length 0 → rejected due to <2 rule.
    assert not _layout(lasers=[(0, (0, 2), Direction.SOUTH)], walls=[(1, 2)], exits=[(2, 2)]).is_geometry_valid()


# ---------------------------------------------------------------------------
# Multiple lasers
# ---------------------------------------------------------------------------
def test_all_lasers_must_be_valid():
    # First laser valid, second points out → whole layout rejected.
    layout = _layout(
        lasers=[
            (0, (1, 0), Direction.EAST),  # valid: long beam
            (1, (0, 2), Direction.NORTH),  # invalid: points out of top row
        ]
    )
    assert not layout.is_geometry_valid()


def test_two_valid_lasers():
    layout = _layout(
        lasers=[
            (0, (0, 1), Direction.SOUTH),
            (1, (0, 3), Direction.SOUTH),
        ]
    )
    assert layout.is_geometry_valid()
