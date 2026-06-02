from collections import defaultdict

from lle import World
from lle.solver._constraints import ConstraintContext, LaserConstraints
from lle.solver.variable_factory import VariableFactory


def test_multiple_same_colour_same_direction_lasers_get_independent_beams():
    # Two colour-0 south lasers in different columns share (colour, direction); each must
    # still keep its own beam, keyed by its distinct source position.
    world = World("""
.  L0S .  L0S .
S0 .   .  .   S1
X  .   .  .   X
""")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 10)
    constraints = LaserConstraints(var, ctx)
    constraints.generate(1)
    # Laser source with id 0 is at (0, 1)
    s1 = world.source_at((0, 1)).laser_id
    assert var.exists("laser", s1, 1, 1, 1)
    assert not var.exists("laser", s1, 1, 3, 1)
    # Laser source with id 1 is at (0, 3)
    s2 = world.source_at((0, 3)).laser_id
    assert var.exists("laser", s2, 1, 3, 1)
    assert not var.exists("laser", s2, 1, 1, 1)


def test_two_same_colour_crossing_lasers_keep_variables():
    # A colour may own many lasers, in several directions, whose beams CROSS. Every laser
    # here is colour 0: three south lasers (columns 1, 2, 3), one east laser (row 1) and one
    # north laser (column 4). The east beam along row 1 crosses all four vertical beams. Each
    # beam must stay independent at every crossing rather than being forced to coincide:
    # a crossing cell carries one beam variable per (direction, source), not a single shared one.
    world = World("""
.   L0S L0S  X
L0E  .   .    .
S0   .   .    .
S1   .   .    .
.    .   .    X
""")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 20)
    constraints = LaserConstraints(var, ctx)
    constraints.generate(10)
    # Make sure that there exist different variables for crossing lasers at i=1
    n_vars_at = defaultdict(lambda: 0)
    for kind, *rest in var.pool.obj2id:
        if kind != "laser":
            continue
        laser_id, i, j, t = rest
        # Count the lasers variables at the intersection
        n_vars_at[i, j, t] += 1
    for (i, j, t), count in n_vars_at.items():
        # Count the actual number of superimposed laser tiles at each position
        n_lasers_superimposed = sum(laser.pos == (i, j) for laser in world.lasers)
        assert count == n_lasers_superimposed
