import pytest
from lle import World
from lle.solver._constraints.base import ConstraintContext


def test_solver_uses_walkable_shortest_path_lower_bound():
    world = World("S0 @ X\n. . .\n. . .")
    ctx = ConstraintContext(world, t_max=10)
    # Manhattan distance would be 2, but the wall forces a 4-step detour.
    assert ctx.solution_lower_bound == 4


def test_context_lower_bound_empty_world():
    N_STEPS = 10
    world_string = "S0 " + " ." * N_STEPS + " X"
    world = World(world_string)
    ctx = ConstraintContext(world, 20)
    assert ctx.solution_lower_bound == N_STEPS + 1


def test_context_lower_bound_with_wall():
    N_STEPS = 10
    world_string = "S0 @ " + ". " * N_STEPS + " X\n"
    world_string += ". " * (N_STEPS + 3)
    world = World(world_string)
    ctx = ConstraintContext(world, 20)
    # +1 for the exit step
    # +1 for the wall
    # +2 for SOUTH and UP
    assert ctx.solution_lower_bound == N_STEPS + 1 + 1 + 2


# ==================== Tests for valid_positions ====================


@pytest.mark.parametrize("level", range(1, 7))
def test_valid_positions_simple(level: int):
    """Test that valid_positions excludes walls and void."""
    world = World.level(level)
    ctx = ConstraintContext(world, 10)

    for i in range(world.height):
        for j in range(world.width):
            pos = (i, j)
            if pos in world.wall_pos or pos in world.void_pos:
                assert pos not in ctx.valid_positions
            else:
                assert pos in ctx.valid_positions


def test_valid_positions_excludes_void():
    """Test that void positions are excluded from valid positions."""
    world = World("""
S0 V
X  V
""")
    ctx = ConstraintContext(world, 10)

    # Check that all void positions are excluded
    assert (0, 1) not in ctx.valid_positions
    assert (1, 1) not in ctx.valid_positions
    assert (0, 0) in ctx.valid_positions
    assert (1, 0) in ctx.valid_positions


# ==================== Tests for exit_reachable ====================


def test_exit_distance_empty():
    world = World("S0 . X")
    ctx = ConstraintContext(world, 10)
    assert ctx._exit_distance[(0, 0)] == 2
    assert ctx._exit_distance[(0, 1)] == 1
    assert ctx._exit_distance[(0, 2)] == 0


def test_exit_distance_walls():
    world = World("S0 @ X\n. . .")
    ctx = ConstraintContext(world, 10)
    assert ctx._exit_distance[(0, 0)] == 4
    assert ctx._exit_distance[(1, 0)] == 3
    assert ctx._exit_distance[(1, 1)] == 2
    assert ctx._exit_distance[(1, 2)] == 1
    assert ctx._exit_distance[(0, 2)] == 0
    assert (0, 1) not in ctx._exit_distance


def test_exit_distance_adjacent_exits_are_zero():
    """Two side-by-side exits must each keep distance 0.

    Regression: when exits are adjacent, each exit is a grid-neighbour of the other.
    A naive forward flood-fill re-enqueued each exit and overwrote its distance with 1,
    poisoning the whole distance map (and thus the reachability pruning, which then made
    the optimal-length horizon spuriously UNSAT).
    """
    world = World("X X\n. .\nS0 S1")
    ctx = ConstraintContext(world, 10)
    assert ctx._exit_distance[(0, 0)] == 0
    assert ctx._exit_distance[(0, 1)] == 0
    # One row below the exits is exactly one step away.
    assert ctx._exit_distance[(1, 0)] == 1
    assert ctx._exit_distance[(1, 1)] == 1
    # Starts are two steps from their exit.
    assert ctx._exit_distance[(2, 0)] == 2
    assert ctx._exit_distance[(2, 1)] == 2


@pytest.mark.parametrize(
    ["map_str", "distance"],
    [
        ("S0 . . . . X", 5),
        ("S0 . . . X\n.  . X . .", 3),
        ("S0 @ . . X\n.  . . X .", 4),
    ],
)
def test_exit_reachable_basic(map_str: str, distance: int):
    """Test that positions at the exit are reachable with any remaining time."""
    T_MAX = 10
    world = World(map_str)
    ctx = ConstraintContext(world, T_MAX)

    for t in range(T_MAX - distance + 1):
        assert (0, 0) in ctx._exit_reachable[t]
    for t in range(T_MAX - distance + 1, T_MAX + 1):
        assert (0, 0) not in ctx._exit_reachable[t]


# ==================== Tests for stay_allowed ====================


@pytest.mark.parametrize(
    ["map_str", "distance"],
    [
        ("S0 . . X", 3),
        ("S0 . . . X\n.  . X . .", 3),
        ("S0 @ . . X\n.  . . X .", 4),
    ],
)
def test_can_stay(map_str: str, distance: int):
    """Test that we can stay at positions with sufficient time to reach exit."""
    T_MAX = 10
    world = World(map_str)
    ctx = ConstraintContext(world, T_MAX)

    for t in range(T_MAX - distance):
        assert ctx.can_stay(t, (0, 0))
    for t in range(T_MAX - distance, T_MAX + 1):
        assert not ctx.can_stay(t, (0, 0))


@pytest.mark.parametrize("level", range(1, 7))
def test_stay_allowed_decreases_with_time(level: int):
    """Test that the set of allowed positions shrinks as time increases."""
    T_MAX = 21
    world = World.level(level)
    ctx = ConstraintContext(world, T_MAX)

    # As time increases, fewer positions allow staying
    n_can_stay = sum([ctx.can_stay(0, (0, j)) for j in range(world.width)])
    for t in range(1, T_MAX):
        next_n_can_stay = sum([ctx.can_stay(t, (0, j)) for j in range(world.width)])
        assert n_can_stay >= next_n_can_stay
        n_can_stay = next_n_can_stay


# ==================== Tests for laser_paths ====================


@pytest.mark.parametrize("level", range(3, 7))
def test_laser_paths_includes_source(level: int):
    """Test that laser paths start with the source position."""
    world = World.level(level)
    ctx = ConstraintContext(world, 21)
    for source in world.laser_sources:
        path = ctx.laser_paths[source]
        assert source.pos not in path


def test_laser_paths_stops_at_walls():
    """Test that laser paths stop when hitting a wall."""
    world = World("""
S0  L0S .
.    .  .
.    @  .
L0E  .  X
""")
    ctx = ConstraintContext(world, 10)

    source = world.source_at((0, 1))
    path = ctx.laser_paths[source]
    assert len(path) == 1
    assert (1, 1) in path

    source = world.source_at((3, 0))
    path = ctx.laser_paths[source]
    assert len(path) == 2
    assert (3, 1) in path
    assert (3, 2) in path


# ==================== Tests for reachable_laser_paths ====================


def test_reachable_laser_paths_simple():
    world = World("""
S0  L0S .
.    .  .
.    .  X
""")
    T_MAX = 10
    ctx = ConstraintContext(world, T_MAX)

    source = world.source_at((0, 1))
    reachable_at_times = ctx.reachable_laser_paths[source]
    assert len(reachable_at_times[0]) == 0
    assert len(reachable_at_times[1]) == 0
    assert len(reachable_at_times[2]) == 1
    assert (1, 1) in reachable_at_times[2]
    for t in range(3, T_MAX - 1):
        assert len(reachable_at_times[t]) == 2
        assert (1, 1) in reachable_at_times[t]
        assert (2, 1) in reachable_at_times[t]
    assert len(reachable_at_times[T_MAX - 1]) == 1
    assert (2, 1) in reachable_at_times[T_MAX - 1]
    assert (1, 1) not in reachable_at_times[T_MAX - 1]


def test_reachable_laser_paths_unblockable():
    world = World("""
S0  @ L0S .
.   @  .  .
.   @  .  X
""")
    T_MAX = 10
    ctx = ConstraintContext(world, T_MAX)

    source = world.source_at((0, 2))
    reachable_at_times = ctx.reachable_laser_paths[source]
    for t in range(T_MAX):
        assert len(reachable_at_times[t]) == 0


def test_reachable_laser_paths_two_agents():
    world = World("""
S0 L1S . . . . . . . . . . . . . S1
.   .  . . . . . . . . . . . . . .
.   .  X . . . . . . . . . . . . X
""")
    DISTANCE = 15
    T_MAX = DISTANCE + 10
    ctx = ConstraintContext(world, T_MAX)

    source = world.source_at((0, 1))
    reachable_at_times = ctx.reachable_laser_paths[source]
    for t in range(DISTANCE):
        assert len(reachable_at_times[t]) == 0
    assert len(reachable_at_times[DISTANCE]) == 1
    assert (1, 1) in reachable_at_times[DISTANCE]
    for t in range(DISTANCE + 1, T_MAX - 1):
        assert len(reachable_at_times[t]) == 2
        assert (1, 1) in reachable_at_times[t]
        assert (2, 1) in reachable_at_times[t]

    assert len(reachable_at_times[T_MAX - 1]) == 1
    assert (1, 1) not in reachable_at_times[T_MAX - 1]
    assert (2, 1) in reachable_at_times[T_MAX - 1]
    assert len(reachable_at_times[T_MAX]) == 0


def test_reachable_lasers_increases_then_decreases_over_time():
    world = World("""
L0E . . . . . . . . . . . . . S0
 .  X . . . . . . . . . . . . .
""")
    T_MAX = 40
    ctx = ConstraintContext(world, T_MAX)

    source = world.source_at((0, 0))
    reachable_at_times = ctx.reachable_laser_paths[source]
    for t in range(13):
        assert len(reachable_at_times[t]) == t + 1
    for i in range(14):
        assert len(reachable_at_times[T_MAX - i]) == i


@pytest.mark.parametrize("level", range(1, 7))
def test_reachable_laser_paths_subset_of_full_path(level: int):
    """Test that reachable positions are subset of full laser path."""
    world = World.level(level)
    ctx = ConstraintContext(world, 21)
    for source in world.laser_sources:
        full_path = set(ctx.laser_paths[source])
        for t in range(ctx.t_max + 1):
            reachable_at_t = set(ctx.reachable_laser_paths[source][t])
            assert reachable_at_t.issubset(full_path)


# ==================== Tests for prev_laser_beam ====================


def test_prev_laser_beam():
    """Test that prev_laser_beam is a dictionary with correct keys."""
    world = World("S0 . . . L0W\nX . . . .")
    ctx = ConstraintContext(world, 10)
    DISTANCE = 4
    for t in range(DISTANCE):
        # Up to t: not None
        for j in range(t):
            assert ctx.get_prev_beam(t, 0, j, 0) == (0, j + 1)
        # t -> end is None
        for j in range(t, DISTANCE):
            assert ctx.get_prev_beam(t, 0, j, 0) is None
    # By the end of the time horizon, the number of relevant lasers decreases
    for t in range(DISTANCE - 1, -1, -1):
        # Up to t: not None
        for j in range(t):
            assert ctx.get_prev_beam(t, 0, j, 0) == (0, j + 1)
        # t -> end is None
        for j in range(t, DISTANCE):
            assert ctx.get_prev_beam(t, 0, j, 0) is None


def test_prev_laser_beam_not_reachable():
    """Test that prev_laser_beam is a dictionary with correct keys."""
    world = World("""
        . . . . L0W
        @ @ @ @  @
        X . . . S0""")
    ctx = ConstraintContext(world, 10)
    for t in range(ctx.t_max):
        for laser in world.lasers:
            x, y = laser.pos
            assert ctx.get_prev_beam(t, x, y, laser.laser_id) is None


def test_prev_laser_beam_not_reachable_within_tmax():
    """Test that prev_laser_beam is a dictionary with correct keys."""
    world = World("""
        . . . . L0W
        @ . @ @  @
        X . . . S0""")
    ctx = ConstraintContext(world, 5)
    for t in range(ctx.t_max):
        for laser in world.lasers:
            x, y = laser.pos
            assert ctx.get_prev_beam(t, x, y, laser.laser_id) is None


def test_prev_laser_beam_not_reachable_because_of_exit():
    """Test that prev_laser_beam is a dictionary with correct keys."""
    world = World("""
        . . . . L0W
        . @ @ @  @
        X . . . S0""")
    ctx = ConstraintContext(world, 10)
    for t in range(ctx.t_max):
        for laser in world.lasers:
            x, y = laser.pos
            assert ctx.get_prev_beam(t, x, y, laser.laser_id) is None


def test_prev_laser_beam_with_crossing_lasers():
    """Test prev_laser_beam with multiple laser sources from same agent."""
    world = World("""
S0 L0S . S1  .
.   .  . .  L1W
X   .  . .   X
""")
    ctx = ConstraintContext(world, 10)
    l0 = world.source_at((0, 1)).laser_id
    l1 = world.source_at((1, 4)).laser_id
    assert ctx.get_prev_beam(0, 1, 1, l0) is None
    assert ctx.get_prev_beam(0, 1, 4, l1) is None
