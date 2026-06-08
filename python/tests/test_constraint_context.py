from lle import World
from lle.solver.constraints_old.context import ConstraintContext

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
