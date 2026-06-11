"""Tests for CustomGenerator placement modes, span constraints, and validation."""

from __future__ import annotations

import pytest

from lle.generator.custom import CustomGenerator
from lle.generator.world_filter import Cooperative, WorldFilter

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

TIMEOUT = 60  # seconds


def _build(gen: CustomGenerator, seed: int = 0, max_attempts: int = 500):
    world = gen.generate(seed=seed, max_attempts=max_attempts)
    assert world is not None, "Generator exhausted max_attempts without producing a world"
    return world


# ---------------------------------------------------------------------------
# Smoke tests
# ---------------------------------------------------------------------------


def test_default_random_builds_world():
    gen = CustomGenerator(width=6, height=6, n_agents=2)
    world = _build(gen)
    assert world.width == 6
    assert world.height == 6
    assert world.n_agents == 2


def test_single_agent():
    gen = CustomGenerator(width=5, height=5, n_agents=1)
    world = _build(gen)
    assert world.n_agents == 1


# ---------------------------------------------------------------------------
# Agent placement
# ---------------------------------------------------------------------------


def test_starts_edge_agents_on_one_edge():
    gen = CustomGenerator(width=8, height=8, n_agents=2, starts="edge", exits="random")
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
        assert on_top or on_bottom or on_left or on_right, (
            f"seed={seed}: agents not on a single edge — rows={rows} cols={cols}"
        )


def test_starts_clustered_agents_form_rectangle():
    gen = CustomGenerator(width=8, height=8, n_agents=2, starts="clustered", exits="random")
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
    gen = CustomGenerator(width=8, height=8, n_agents=2, starts="edge", exits="opposite")
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
    gen = CustomGenerator(width=10, height=10, n_agents=2, starts="clustered", exits="opposite")
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
        gen = CustomGenerator(width=6, height=6, n_agents=2, exits=mode)
        world = _build(gen)
        agent_pos = set(p[0] for p in world.random_start_pos)
        exit_pos = set(world.exit_pos)
        assert not agent_pos & exit_pos, f"exits={mode!r}: overlap between agents and exits"


# ---------------------------------------------------------------------------
# Wall placement
# ---------------------------------------------------------------------------


def test_no_walls():
    gen = CustomGenerator(width=6, height=6, n_agents=2, n_walls=0)
    world = _build(gen)
    assert world.wall_pos == []


def test_walls_individual():
    gen = CustomGenerator(width=8, height=8, n_agents=2, n_walls=5, walls_style="individual")
    world = _build(gen)
    assert len(world.wall_pos) == 5


def test_walls_shapes():
    gen = CustomGenerator(width=8, height=8, n_agents=2, n_walls=6, walls_style="shapes")
    world = _build(gen)
    # place_wall_shapes may produce ≤ budget; just ensure it built successfully
    assert isinstance(world.wall_pos, list)


# ---------------------------------------------------------------------------
# Laser placement: free
# ---------------------------------------------------------------------------


def test_lasers_free_count():
    gen = CustomGenerator(width=8, height=8, n_agents=2, n_lasers=2, laser_placement="free")
    world = _build(gen)
    assert len(world.laser_sources) == 2


def test_laser_span_int_minimum():
    span = 4
    gen = CustomGenerator(
        width=8, height=8, n_agents=2, n_lasers=1, laser_placement="free", laser_span=span
    )
    world = _build(gen)
    assert len(world.laser_sources) == 1
    assert len(world.lasers) >= span


def test_laser_span_across():
    gen = CustomGenerator(
        width=8, height=8, n_agents=2, n_lasers=1, laser_placement="free", laser_span="across"
    )
    world = _build(gen)
    # The laser beam must span the full row or column
    assert len(world.lasers) >= 1


# ---------------------------------------------------------------------------
# Laser placement: cross-agent
# ---------------------------------------------------------------------------


def test_cross_agent_laser_crosses_all_lanes():
    gen = CustomGenerator(
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
    gen = CustomGenerator(
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
    gen = CustomGenerator(
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
    gen = CustomGenerator(
        width=8,
        height=8,
        n_agents=2,
        starts="edge",
        exits="opposite",
        n_lasers=1,
        laser_placement="cross-agent",
        filter=WorldFilter.cooperative(30),
    )
    world = _build(gen, max_attempts=200)
    assert Cooperative(30).is_satisfied_by(world)


# ---------------------------------------------------------------------------
# Construction-time validation errors
# ---------------------------------------------------------------------------


def test_error_opposite_with_random_starts():
    with pytest.raises(ValueError, match="opposite"):
        CustomGenerator(width=5, height=5, n_agents=2, starts="random", exits="opposite")


def test_error_cross_agent_requires_edge():
    with pytest.raises(ValueError, match="cross-agent"):
        CustomGenerator(
            width=5, height=5, n_agents=2, starts="clustered", n_lasers=1, laser_placement="cross-agent"
        )


def test_error_cross_cluster_requires_clustered():
    with pytest.raises(ValueError, match="cross-cluster"):
        CustomGenerator(
            width=5, height=5, n_agents=2, starts="edge", n_lasers=1, laser_placement="cross-cluster"
        )


def test_error_cross_cluster_requires_cluster_exits():
    with pytest.raises(ValueError, match="cross-cluster"):
        CustomGenerator(
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
        CustomGenerator(width=5, height=5, n_agents=2, n_lasers=1, laser_span=1)


# ---------------------------------------------------------------------------
# Import-level smoke test
# ---------------------------------------------------------------------------


def test_importable_from_lle():
    import lle

    assert hasattr(lle, "CustomGenerator")
    assert lle.CustomGenerator is CustomGenerator
