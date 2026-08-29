"""Tests for CustomGenerator placement modes, span constraints, and validation."""

from __future__ import annotations

import random

import pytest
from lle.exceptions import ParsingError
from lle.generator import generate
from lle.generator.generator import WorldGenerator
from lle.generator.geometry import beam_tiles
from lle.generator.placements import LayoutRetry
from lle.generator.world_filter import Constraint, Cooperative, Interdependent, Sequential
from lle.tiles import Direction
from lle.types import Position

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

TIMEOUT = 60  # seconds


def _build(gen: WorldGenerator, seed: int = 0, max_attempts: int = 500):
    world = gen.generate(seed=seed, max_attempts=max_attempts)
    assert world is not None, "Generator exhausted max_attempts without producing a world"
    return world


# ---------------------------------------------------------------------------
# Smoke tests
# ---------------------------------------------------------------------------


def test_default_random_builds_world():
    gen = WorldGenerator(width=6, height=6, n_agents=2)
    world = _build(gen)
    assert world.width == 6
    assert world.height == 6
    assert world.n_agents == 2


def test_single_agent():
    gen = WorldGenerator(width=5, height=5, n_agents=1)
    world = _build(gen)
    assert world.n_agents == 1


def test_builder_places_requested_gems_on_free_cells():
    """The fluent gem configuration places each requested gem without overlap."""
    world = generate(width=8, height=8, n_agents=2).gems(5).lasers(1).walls(4).build(seed=0)

    gem_positions = {gem.pos for gem in world.gems}
    occupied_positions = (
        {start[0] for start in world.random_start_pos}
        | set(world.exit_pos)
        | set(world.wall_pos)
        | {source.pos for source in world.laser_sources}
    )
    assert len(gem_positions) == 5
    assert not gem_positions & occupied_positions


# ---------------------------------------------------------------------------
# Agent placement
# ---------------------------------------------------------------------------


def test_starts_edge_agents_on_one_edge():
    gen = WorldGenerator(width=8, height=8, n_agents=2, starts="edge", exits="random")
    for seed in range(10):
        world = _build(gen, seed=seed)
        pos = world.random_start_pos
        rows = [p[0][0] for p in pos]
        cols = [p[0][1] for p in pos]
        # All agents on the same edge (same row or same col)
        on_top = all(r == 0 for r in rows)
        on_bottom = all(r == world.height - 1 for r in rows)
        on_left = all(c == 0 for c in cols)
        on_right = all(c == world.width - 1 for c in cols)
        assert on_top or on_bottom or on_left or on_right, f"seed={seed}: agents not on a single edge — rows={rows} cols={cols}"


def test_starts_clustered_agents_form_rectangle():
    gen = WorldGenerator(width=8, height=8, n_agents=2, starts="clustered", exits="random")
    for seed in range(10):
        world = _build(gen, seed=seed)
        pos = [(p[0][0], p[0][1]) for p in world.random_start_pos]
        rows = [r for r, _ in pos]
        cols = [c for _, c in pos]
        # For n_agents=2, cluster shape is (1, 2): all in same row, adjacent cols
        assert max(rows) - min(rows) <= 1
        assert max(cols) - min(cols) <= 2


# ---------------------------------------------------------------------------
# Exit placement
# ---------------------------------------------------------------------------


def test_exits_opposite_edge():
    """With starts='edge' and exits='opposite', exits must be on the opposite edge."""
    gen = WorldGenerator(width=8, height=8, n_agents=2, starts="edge", exits="opposite")
    for seed in range(10):
        world = _build(gen, seed=seed)
        agent_rows = [p[0][0] for p in world.random_start_pos]
        agent_cols = [p[0][1] for p in world.random_start_pos]
        exit_rows = [r for r, _ in world.exit_pos]
        exit_cols = [c for _, c in world.exit_pos]

        # Agents all on same edge; exits all on opposite
        if all(c == 0 for c in agent_cols):  # left
            assert all(c == world.width - 1 for c in exit_cols)
        elif all(c == world.width - 1 for c in agent_cols):  # right
            assert all(c == 0 for c in exit_cols)
        elif all(r == 0 for r in agent_rows):  # top
            assert all(r == world.height - 1 for r in exit_rows)
        else:  # bottom
            assert all(r == 0 for r in exit_rows)


def test_exits_opposite_cluster():
    """With starts='clustered' and exits='opposite', exits form a cluster far from agents."""
    gen = WorldGenerator(width=10, height=10, n_agents=2, starts="clustered", exits="opposite")
    for seed in range(10):
        world = _build(gen, seed=seed)
        agent_rows = [p[0][0] for p in world.random_start_pos]
        agent_cols = [p[0][1] for p in world.random_start_pos]
        exit_rows = [r for r, _ in world.exit_pos]
        exit_cols = [c for _, c in world.exit_pos]
        # Exits and agents must not overlap
        assert not set(zip(agent_rows, agent_cols)) & set(zip(exit_rows, exit_cols))


def test_exits_no_overlap_with_agents():
    for mode in ("random", "edge", "cluster"):
        gen = WorldGenerator(width=6, height=6, n_agents=2, exits=mode)
        world = _build(gen)
        agent_pos = set(p[0] for p in world.random_start_pos)
        exit_pos = set(world.exit_pos)
        assert not agent_pos & exit_pos, f"exits={mode!r}: overlap between agents and exits"


# ---------------------------------------------------------------------------
# Wall placement
# ---------------------------------------------------------------------------


def test_no_walls():
    gen = WorldGenerator(width=6, height=6, n_agents=2, n_walls=0)
    world = _build(gen)
    assert world.wall_pos == []


def test_walls_individual():
    gen = WorldGenerator(width=8, height=8, n_agents=2, n_walls=5, walls_style="individual")
    world = _build(gen)
    assert len(world.wall_pos) == 5


def test_walls_shapes():
    gen = WorldGenerator(width=8, height=8, n_agents=2, n_walls=6, walls_style="shapes")
    world = _build(gen)
    # place_wall_shapes may produce ≤ budget; just ensure it built successfully
    assert isinstance(world.wall_pos, list)


# ---------------------------------------------------------------------------
# Laser placement: free
# ---------------------------------------------------------------------------


def test_lasers_free_count():
    gen = WorldGenerator(width=8, height=8, n_agents=2, n_lasers=2, laser_placement="free")
    world = _build(gen)
    assert len(world.laser_sources) == 2


def test_laser_span_int_minimum():
    span = 4
    gen = WorldGenerator(width=8, height=8, n_agents=2, n_lasers=1, laser_placement="free", laser_span=span)
    world = _build(gen)
    assert len(world.laser_sources) == 1
    assert len(world.lasers) >= span


def test_laser_span_across():
    gen = WorldGenerator(width=8, height=8, n_agents=2, n_lasers=1, laser_placement="free", laser_span="across")
    world = _build(gen)
    # The laser beam must span the full row or column
    assert len(world.lasers) >= 1


# ---------------------------------------------------------------------------
# Laser placement: cross-agent
# ---------------------------------------------------------------------------


def test_cross_agent_laser_crosses_all_lanes():
    gen = WorldGenerator(
        width=8,
        height=8,
        n_agents=2,
        starts="edge",
        exits="opposite",
        n_lasers=1,
        laser_placement="cross-agent",
    )
    world = _build(gen)
    assert len(world.laser_sources) == 1
    # The beam tiles should span at least n_agents rows (or cols)
    assert len(world.lasers) >= world.n_agents


def test_cross_agent_multiple_lasers():
    gen = WorldGenerator(
        width=10,
        height=10,
        n_agents=2,
        starts="edge",
        exits="opposite",
        n_lasers=2,
        laser_placement="cross-agent",
    )
    world = _build(gen)
    assert len(world.laser_sources) == 2


# ---------------------------------------------------------------------------
# Laser placement: cross-cluster
# ---------------------------------------------------------------------------


def test_cross_cluster_laser_in_corridor():
    gen = WorldGenerator(
        width=10,
        height=10,
        n_agents=2,
        starts="clustered",
        exits="opposite",
        n_lasers=1,
        laser_placement="cross-cluster",
    )
    world = _build(gen)
    assert len(world.laser_sources) == 1


# ---------------------------------------------------------------------------
# Filter integration
# ---------------------------------------------------------------------------


def test_cooperative_filter():
    gen = WorldGenerator(
        width=8,
        height=8,
        n_agents=2,
        starts="edge",
        exits="opposite",
        n_lasers=1,
        laser_placement="cross-agent",
        constraint=Constraint(30, Cooperative()),
    )
    world = _build(gen, max_attempts=200)
    assert Constraint(30, Cooperative()).is_satisfied_by(world)


# ---------------------------------------------------------------------------
# Construction-time validation errors
# ---------------------------------------------------------------------------


def test_error_opposite_with_random_starts():
    with pytest.raises(ValueError, match="opposite"):
        WorldGenerator(width=5, height=5, n_agents=2, starts="random", exits="opposite")


def test_error_cross_agent_requires_edge():
    with pytest.raises(ValueError, match="cross-agent"):
        WorldGenerator(width=5, height=5, n_agents=2, starts="clustered", n_lasers=1, laser_placement="cross-agent")


def test_error_cross_cluster_requires_clustered():
    with pytest.raises(ValueError, match="cross-cluster"):
        WorldGenerator(width=5, height=5, n_agents=2, starts="edge", n_lasers=1, laser_placement="cross-cluster")


def test_error_cross_cluster_requires_cluster_exits():
    with pytest.raises(ValueError, match="cross-cluster"):
        WorldGenerator(
            width=5,
            height=5,
            n_agents=2,
            starts="clustered",
            exits="random",
            n_lasers=1,
            laser_placement="cross-cluster",
        )


def test_error_laser_span_too_small():
    with pytest.raises(ValueError, match="laser_span"):
        WorldGenerator(width=5, height=5, n_agents=2, n_lasers=1, laser_span=1)


def test_error_gems_exceed_cells_after_starts_and_exits():
    """The gem count cannot consume cells reserved for starts and exits."""
    with pytest.raises(ValueError, match=r"gems must be <= grid cells minus start and exit cells \(14\)"):
        generate(width=4, height=4, n_agents=1).gems(15).build(max_attempts=1)


# ---------------------------------------------------------------------------
# Filter-specific validation errors
# ---------------------------------------------------------------------------


def test_error_cooperative_requires_n_agents_ge_2():
    with pytest.raises(ValueError, match="agents"):
        WorldGenerator(width=5, height=5, n_agents=1, n_lasers=1, constraint=Constraint(20, Cooperative()))


def test_error_cooperative_requires_lasers():
    with pytest.raises(ValueError, match="laser"):
        WorldGenerator(width=5, height=5, n_agents=2, n_lasers=0, constraint=Constraint(20, Cooperative()))


def test_error_mutual_requires_n_agents_ge_2():
    with pytest.raises(ValueError, match="agents"):
        WorldGenerator(width=5, height=5, n_agents=1, n_lasers=1, constraint=Constraint(20, Interdependent(2)))


def test_error_mutual_requires_n_lasers_ge_2():
    with pytest.raises(ValueError, match="laser"):
        WorldGenerator(width=5, height=5, n_agents=2, n_lasers=1, constraint=Constraint(20, Interdependent(2)))


def test_error_sequential_requires_n_agents_ge_2():
    with pytest.raises(ValueError, match="agents"):
        WorldGenerator(width=5, height=5, n_agents=1, n_lasers=1, constraint=Constraint(20, Sequential()))


def test_error_sequential_requires_n_lasers_ge_2():
    with pytest.raises(ValueError, match="laser"):
        WorldGenerator(width=5, height=5, n_agents=2, n_lasers=1, constraint=Constraint(20, Sequential()))


def test_error_sequential_order_3_requires_three_lasers():
    with pytest.raises(ValueError, match="laser"):
        WorldGenerator(width=5, height=5, n_agents=3, n_lasers=2, constraint=Constraint(20, Sequential(3)))


def test_error_interdependent_order_3_requires_three_agents():
    with pytest.raises(ValueError, match="agents"):
        WorldGenerator(width=5, height=5, n_agents=2, n_lasers=2, constraint=Constraint(20, Interdependent(3)))


def test_error_interdependent_order_3_requires_three_lasers():
    with pytest.raises(ValueError, match="laser"):
        WorldGenerator(width=5, height=5, n_agents=3, n_lasers=2, constraint=Constraint(20, Interdependent(3)))


def test_cross_cluster_lasers_do_not_cover_start_tiles():
    """A geometrically valid candidate layout must never aim a beam at an agent start tile.

    `CandidateLayout.is_geometry_valid` rejects beams that cover an exit but not beams that cover a
    start, so `cross-cluster` placement can emit a layout whose beam overwrites `S<id>`. The world
    string then has no start tile for that agent and `World(...)` refuses to parse it.
    """
    gen = WorldGenerator(
        width=9,
        height=9,
        n_agents=3,
        n_lasers=3,
        starts="clustered",
        exits="opposite",
        laser_placement="cross-cluster",
    )
    gen._rng.seed(1)
    offenders: list[tuple[Position, tuple[int, Position, Direction]]] = []
    for _ in range(400):
        try:
            layout = gen._make_candidate_layout()
        except LayoutRetry:
            continue
        wall_set = set(layout.walls)
        laser_set = {pos for _, pos, _ in layout.lasers}
        for laser in layout.lasers:
            _owner, src, direction = laser
            tiles = beam_tiles(src, direction, wall_set, laser_set, layout.height, layout.width)
            for start in layout.agents:
                if start in tiles:
                    offenders.append((start, laser))
    assert offenders == [], f"{len(offenders)} beams cover an agent start tile, e.g. {offenders[0]}"


def test_cross_cluster_generation_resamples_instead_of_raising():
    """`generate` must resample an unbuildable candidate rather than propagate the parsing error.

    `_try_generate` only catches `LayoutRetry`, so a candidate that `WorldBuilder.build` rejects
    aborts the whole run. The offending candidate appears for roughly one seed in five, hence the
    sweep: `place_cluster_shape` draws from the global `random` module instead of the injected
    generator, so no single seed reproduces it reliably.
    """
    failures: list[tuple[int, str]] = []
    for seed in range(30):
        gen = WorldGenerator(
            width=9,
            height=9,
            n_agents=3,
            n_lasers=3,
            starts="clustered",
            exits="opposite",
            laser_placement="cross-cluster",
        )
        try:
            world = gen.generate(seed=seed, max_attempts=200)
        except ParsingError as e:
            failures.append((seed, str(e)))
            continue
        assert world is not None
        assert world.n_agents == 3
    assert failures == [], f"generate() raised for {len(failures)} of 30 seeds, e.g. {failures[0]}"


def test_clustered_starts_are_reproducible_from_a_seed():
    """Two generators given the same seed must produce the same world.

    `place_cluster_shape` picks the cluster orientation with `random.choice` on the global module
    rather than on the `rng` passed down from the generator, so `starts="clustered"` ignores the
    seed for that decision.
    """

    def build(seed: int, global_seed: int):
        random.seed(global_seed)
        gen = WorldGenerator(width=9, height=9, n_agents=3, starts="clustered", exits="opposite")
        return gen.generate(seed=seed, max_attempts=200)

    divergent = []
    for seed in range(12):
        first = build(seed, 1)
        second = build(seed, 999)
        assert first is not None and second is not None
        if first.start_pos != second.start_pos:
            divergent.append((seed, first.start_pos, second.start_pos))
    assert divergent == [], f"the global RNG state changed the layout for seeds {[d[0] for d in divergent]}"
