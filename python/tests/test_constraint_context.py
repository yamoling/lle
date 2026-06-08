import pytest
from lle import World
from lle.solver.constraints_old.context import ConstraintContext

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
