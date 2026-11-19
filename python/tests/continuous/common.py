"""Maps and helpers shared by the tests of `lle.continuous`.

Conventions (design doc §4.1): a cell `(i, j)` is the square `[j, j + 1] × [i, i + 1]` and its
centre is `(j + 0.5, i + 0.5)`. Continuous positions are `(x, y)`, with `x` eastwards and `y`
southwards. An action `(ax, ay)` is in the same order: `ax` east, `ay` south, so North is `-y`.

Default physics: `dt = 0.05`, `max_speed = 2.0`, hence `0.1` cell per tick at full speed, and
`agent_radius = 0.3`, `gem_radius = 0.25`, `beam_width = 0.1`.
"""

from collections.abc import Iterable, Sequence

from lle import Action

STILL = (0.0, 0.0)
EAST = (1.0, 0.0)
WEST = (-1.0, 0.0)
NORTH = (0.0, -1.0)
SOUTH = (0.0, 1.0)

TWO_AGENTS = """
S0 S1 G
X  X  L0N
"""
"""Width 3, height 2. Agents 0 (colour 0) at (0.5, 0.5) and 1 (colour 1) at (1.5, 0.5). One gem on
the cell (0, 2), exits on (1, 0) and (1, 1). One source L0N on (1, 2), whose beam covers the cell
(0, 2) only, then leaves the grid. Nobody touches the beam at x = 2.5."""

CORRIDOR = "S0 . . . . . . X"
"""A single agent that can walk east for 6.5 cells without any event."""

GEM_THEN_EXIT = "S0 G X"
"""At speed 0.7 (0.07 cell per tick): the gem at tick index 6 (x = 0.99 > 0.5 + 0.45), the exit at
tick index 21 (x = 2.04). At full speed: the gem at tick index 4, the exit at tick index 14."""

INTO_VOID = "S0 V X"
"""Walking east at full speed, the disc overlaps the void (x > 1 - r) at tick index 2 (x = 0.8)."""

RANDOM_STARTS = """
world_string = \"\"\"
. . . X
. . . X
\"\"\"
[[agents]]
start_positions = [{ i_min = 0, i_max = 1, j_min = 0, j_max = 2 }]
[[agents]]
start_positions = [{ i_min = 0, i_max = 1, j_min = 0, j_max = 2 }]
"""
"""Both agents start on one of the 6 cells of rows 0–1 and columns 0–2, never the same one."""

SAME_COLOUR = """
world_string = \"\"\"
S0  .  . X
L0E .  . .
.   S1 . X
\"\"\"
[[agents]]
colour = 0
[[agents]]
colour = 0
"""
"""Both agents have colour 0, as the only source: `n_colours` is 1, not `n_agents`."""


def xy(points: Iterable[Sequence[float]]) -> list[tuple[float, float]]:
    """Normalize a sequence of `(x, y)` pairs (tuples or lists) to a list of tuples."""
    return [(float(p[0]), float(p[1])) for p in points]


def cells(positions: Iterable[Sequence[int]]) -> list[tuple[int, ...]]:
    """
    Normalize a sequence of positions (plain tuples or `lle` position objects) to plain tuples.

    The `lle` position objects compare equal to tuples but do not hash like them, so sets of cells
    must be built from this normalized form.

    @ai-generated
    """
    return [tuple(int(c) for c in p) for p in positions]


def tagged_events(outcome) -> list[tuple[int, object, int | None]]:
    """
    The `(tick, event_type, agent_id)` triples of a `StepOutcome`, in order.

    @ai-generated
    """
    assert len(outcome.events) == len(outcome.ticks)
    return [(tick, e.event_type, e.agent_id) for tick, e in zip(outcome.ticks, outcome.events)]


def velocity_of(action: Action) -> tuple[float, float]:
    """
    The unit velocity command `(ax, ay)` matching a discrete action: `delta` is `(di, dj)`, and
    `(x, y)` is `(j, i)`. `STAY` gives `(0, 0)`.

    @ai-generated
    """
    di, dj = action.delta
    return (float(dj), float(di))
