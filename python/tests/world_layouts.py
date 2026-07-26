from __future__ import annotations

from dataclasses import KW_ONLY, dataclass, field
from typing import Iterable, Literal

from lle import World

ScalarPropertyName = Literal[
    "solvable",
    "cooperative",
    "independent",
    "asymmetric",
    "fully_coupled",
]


@dataclass(frozen=True)
class ExpectedProperties:
    solvable: bool | None = None
    cooperative: bool | None = None
    independent: bool | None = None
    asymmetric: bool | None = None
    fully_coupled: bool | None = None
    convergent: dict[int, bool] = field(default_factory=dict)
    divergent: dict[int, bool] = field(default_factory=dict)
    chained: dict[int, bool] = field(default_factory=dict)
    interdependent: dict[int, bool] = field(default_factory=dict)


@dataclass(frozen=True)
class Expectation:
    t_max: int
    properties: ExpectedProperties


@dataclass(frozen=True)
class Layout:
    name: str
    source: str | int
    expectations: tuple[Expectation, ...]
    _: KW_ONLY
    description: str = ""

    def world(self):
        """Build a fresh world from the shared layout definition."""
        if isinstance(self.source, int):
            return World.level(self.source)
        return World(self.source)


@dataclass(frozen=True)
class ScalarPropertyCase:
    layout: Layout
    t_max: int
    expected: bool

    @property
    def id(self):
        return f"{self.layout.name}-t{self.t_max}"


@dataclass(frozen=True)
class ChainedCase:
    layout: Layout
    t_max: int
    expected: bool
    length: int

    @property
    def id(self):
        return f"{self.layout.name}-t{self.t_max}-{self.length}"


@dataclass(frozen=True)
class ConvergentCase:
    layout: Layout
    t_max: int
    expected: bool
    k: int

    @property
    def id(self):
        return f"{self.layout.name}-t{self.t_max}-{self.k}"


@dataclass(frozen=True)
class DivergentCase:
    layout: Layout
    t_max: int
    expected: bool
    k: int

    @property
    def id(self):
        return f"{self.layout.name}-t{self.t_max}-{self.k}"


@dataclass(frozen=True)
class InterdependentCase:
    layout: Layout
    t_max: int
    expected: bool
    order: int

    @property
    def id(self):
        return f"{self.layout.name}-t{self.t_max}-{self.order}"


def expect(
    t_max: int,
    *,
    solvable: bool | None = None,
    cooperative: bool | None = None,
    independent: bool | None = None,
    asymmetric: bool | None = None,
    fully_coupled: bool | None = None,
    convergent: dict[int, bool] | None = None,
    divergent: dict[int, bool] | None = None,
    chained: dict[int, bool] | None = None,
    interdependent: dict[int, bool] | None = None,
) -> Expectation:
    """Declare the properties expected from one layout at one horizon."""
    if cooperative is not None and independent is not None:
        assert cooperative != independent, "cooperative and independent cannot be equal"
    elif cooperative is not None:
        independent = not cooperative
    elif independent is not None:
        cooperative = not independent
    return Expectation(
        t_max,
        ExpectedProperties(
            solvable=solvable,
            cooperative=cooperative,
            independent=independent,
            asymmetric=asymmetric,
            fully_coupled=fully_coupled,
            convergent={} if convergent is None else convergent,
            divergent={} if divergent is None else divergent,
            chained={} if chained is None else chained,
            interdependent={} if interdependent is None else interdependent,
        ),
    )


def expect_for(
    horizons: Iterable[int],
    *,
    solvable: bool | None = None,
    cooperative: bool | None = None,
    independent: bool | None = None,
    asymmetric: bool | None = None,
    fully_coupled: bool | None = None,
    convergent: dict[int, bool] | None = None,
    divergent: dict[int, bool] | None = None,
    chained: dict[int, bool] | None = None,
    interdependent: dict[int, bool] | None = None,
) -> tuple[Expectation, ...]:
    """Declare the same property matrix for several horizons."""
    return tuple(
        expect(
            t_max,
            solvable=solvable,
            cooperative=cooperative,
            independent=independent,
            asymmetric=asymmetric,
            fully_coupled=fully_coupled,
            convergent=convergent,
            divergent=divergent,
            chained=chained,
            interdependent=interdependent,
        )
        for t_max in horizons
    )


# Built-in levels

LEVEL_1 = Layout(
    "level-1",
    1,
    (expect(10, solvable=True, cooperative=False, independent=True),),
)

LEVEL_2 = Layout(
    "level-2",
    2,
    (expect(10, cooperative=False, independent=True),),
)

LEVEL_3 = Layout(
    "level-3",
    3,
    (
        *expect_for(range(10), solvable=False),
        *expect_for(range(10, 20), cooperative=True, asymmetric=True),
    ),
)

LEVEL_4 = Layout(
    "level-4",
    4,
    (expect(10, cooperative=True, interdependent={2: True}),),
)

LEVEL_5 = Layout(
    "level-5",
    5,
    (
        expect(19, cooperative=True),
        expect(21, asymmetric=True),
        expect(25, asymmetric=False, interdependent={2: False}),
    ),
)

LEVEL_6 = Layout(
    "level-6",
    6,
    (
        expect(
            21,
            solvable=True,
            cooperative=True,
            independent=False,
            chained={
                2: True,
                3: False,
            },
            interdependent={2: True, 3: False},
            divergent={2: True, 3: True, 4: False},
        ),
    ),
)


# Solvability and independent movement

BLOCKED_UNSOLVABLE = Layout(
    "blocked-unsolvable",
    "S0 @ X",
    (expect(10, solvable=False),),
)

UNSOLVABLE_4AGENTS = Layout(
    "unsolvable-4agents",
    """S0 S1 S2 S3 X X X X""",
    (expect(10, solvable=False),),
)

OPEN_TWO_AGENT = Layout(
    "open-two-agent",
    """
S0 . S1
 . . .
 X . X
""",
    (
        expect(
            6,
            solvable=True,
            cooperative=False,
            independent=True,
            asymmetric=False,
            fully_coupled=False,
            convergent={2: False},
            divergent={2: False},
            interdependent={2: False},
        ),
    ),
)

OPEN_TWO_AGENT_WIDE = Layout(
    "open-two-agent-wide",
    """
 . . . . X
S0 . . . .
S1 . . . .
 . . . . X
""",
    (expect(10, solvable=True, cooperative=False),),
)


ONE_WAY_DETOUR = Layout(
    "one-way-detour",
    """
 .  . S0 S1 . .
L0E .  .  . @ .
 .  .  .  . . .
 .  .  .  . . .
 X  X  .  . . .
""",
    (
        *expect_for(
            range(6, 10),
            solvable=True,
            cooperative=True,
            independent=False,
            asymmetric=True,
            chained={2: False, 3: False},
            interdependent={2: False, 3: False},
        ),
        *expect_for(
            range(10, 12),
            solvable=True,
            cooperative=False,
            independent=True,
            asymmetric=False,
            chained={2: False, 3: False},
            interdependent={2: False, 3: False},
        ),
    ),
    description="A one-way help edge is avoidable through an independent detour from t=10.",
)

SINGLE_LASER_ASYMMETRIC = Layout(
    "single-laser-asymmetric",
    """
 @  S0 S1
L0E .  .
 @  X  X
""",
    (
        expect(
            6,
            solvable=True,
            cooperative=True,
            asymmetric=True,
            convergent={2: False},
            divergent={2: False},
            chained={2: False},
            interdependent={2: False},
        ),
    ),
)

DOUBLE_DISJOINT_ASYMMETRIC = Layout(
    "double-disjoint-asymmetric",
    """
 @  S0 S1 @  @  S2 S3
L0E .  .  @ L2E .  .
 @  X  X  @  @  X  X
""",
    (expect(6, asymmetric=True, convergent={2: False}, divergent={2: False}),),
)

CONVERGENT_2_TIGHT = Layout(
    "convergent-2-tight",
    """
 @  S0  @  @  @  S2
L0E  .  .  .  .  .
 @   X  @  @  @  .
L1E  .  .  .  .  .
 @  S1  @  @  @  .
 @   X  @  @  @  X
""",
    (
        expect(
            5,
            solvable=True,
            cooperative=True,
            convergent={2: True, 3: False},
            divergent={2: False},
        ),
    ),
    description="Agent 2 must receive sequential help from agents 0 and 1 within five steps.",
)

DIVERGENT_2_TIGHT = Layout(
    "divergent-2-tight",
    """
 @   X   X   X  @
L0E  .   .   .  .
 @  S0  S1  S2  @
""",
    (
        *expect_for(
            (2, 8),
            convergent={2: False},
            asymmetric=True,
            divergent={2: True, 3: False},
        ),
    ),
    description="Agent 0 must block its own beam for agents 1 and 2 at once; the flattened help "
    "relation of the two-step witness is exactly {(0, 1), (0, 2)}. Checked at a larger horizon "
    "too, so the expectation is not a shortest-plan artefact.",
)

DIVERGENT_2_WITH_DETOUR = Layout(
    "divergent-2-with-detour",
    """
 @   X   X   X  @   X
L0E  .   .   .  @   .
 @  S0  S1  S2  @   .
 @   @   @   .   .  .
""",
    (
        *expect_for((2, 5), solvable=True, cooperative=True, divergent={2: True}),
        *expect_for((6, 8), solvable=True, cooperative=True, divergent={2: False}),
    ),
    description="Same divergent crossing as the tight layout, but agent 2 also has a six-step "
    "route around the wall, so required divergence disappears at t_max=6.",
)

CHAIN_4_WITH_MUTUAL = Layout(
    "chain-4-with-mutual",
    """
 @  S0 S1  @
L0E X  .   @
 @  S2 .   @
 @  .  X  L1W
 @  .  S3  @
L2E .  .   @
 @  X  X  L3W
""",
    (
        expect(
            6,
            solvable=True,
            chained={2: True, 3: True, 4: True, 5: False},
            interdependent={2: True, 3: False, 4: False},
        ),
    ),
)

CHAIN_3_WITHOUT_CYCLE = Layout(
    "chain-3-without-cycle",
    """
 @  S0 S1  @
L0E X  .   @
 @  S2 .   @
 @  .  X  L1W
 @  .  S3  @
L2E X  .   @
 @  .  X   @
""",
    (
        expect(
            6,
            solvable=True,
            chained={2: True, 3: True, 4: False},
            interdependent={2: False, 3: False, 4: False},
        ),
    ),
)

PAPER_CHAIN_2 = Layout(
    "paper-chain-2",
    """
 @  S0 @ S1 @
L0E .  . .  @
 @  X  @ . S2
 @ L1E . .  .
 @  @  @ X  x
""",
    (
        expect(
            10,
            solvable=True,
            chained={2: True, 3: False},
            convergent={2: False},
            interdependent={2: False},
        ),
    ),
)

PAPER_CHAIN_2_NOT_INTERDEPENDENT_3 = Layout(
    "paper-chain-2-not-interdependent-3",
    """
 @  S0 @  @  L1S S1
L0E .  .  .   .  .
 @  . S2 L2W  .  X
 @  .  .  .   .  X
L0E .  @  .   .  .
 @  X  @  .   .  .
""",
    (
        expect(
            10,
            solvable=True,
            chained={2: True, 3: False},
            convergent={2: False},
            interdependent={2: False, 3: False},
        ),
    ),
)

PAPER_CONVERGENT_2 = Layout(
    "paper-convergent-2",
    """
 @   S0  .  S2  .
L0E  .   .  .   @
 @   X   @  .   .
 @  L1E  .  S1  .
 @   @   @  X   X
""",
    (
        expect(
            10,
            solvable=True,
            convergent={2: True, 3: False},
            divergent={2: False},
            chained={2: False},
            interdependent={2: False},
            asymmetric=True,
        ),
    ),
)

PAPER_FULLY_COUPLED = Layout(
    "paper-fully-coupled",
    """
 @  L0S  @ @ @ @
S0   .   . . @ @
S1   .   . . . @
S2   .   . . . @
 @  L2E  . . . @
 @   @   X X X L1W
""",
    (
        expect(
            10,
            solvable=True,
            cooperative=True,
            independent=False,
            asymmetric=False,
            fully_coupled=True,
            convergent={2: True, 3: False},
            divergent={2: True, 3: False},
            interdependent={2: True, 3: True, 4: False},
        ),
    ),
)

PAPER_FULLY_COUPLED_LEGACY = Layout(
    "paper-fully-coupled-legacy",
    """
 .  S0 S1 S2 .
L0E .  .  .  .
 .  .  .  . L2W
L1E .  .  .  .
 .  X  X  X  .
""",
    (
        expect(
            10,
            solvable=True,
            cooperative=True,
            independent=False,
            asymmetric=False,
            fully_coupled=True,
            interdependent={2: True, 3: True, 4: False},
        ),
    ),
    description="This is a fully coupled example that was used previously in the paper but was removed because the trajectories were unreadable.",
)

TWO_AGENT_MUTUAL_COMPACT = Layout(
    "two-agent-mutual-compact",
    """
 S0 . . S1
L0E . . .
 .  . . L1W
 X  . . X
""",
    (expect(6, asymmetric=False, interdependent={2: True}),),
)

TWO_AGENT_MUTUAL_WITH_DETOURS = Layout(
    "two-agent-mutual-with-detours",
    """
 .  . . S0 S1  .  . . .
L0E . .  .  .  @  @ @ .
 .  . @  .  . L1W . . .
 .  . .  .  .  .  . . .
 .  . .  X  X  .  . . .
""",
    (
        expect(
            5,
            solvable=True,
            cooperative=True,
            independent=False,
            interdependent={2: True},
        ),
        expect(
            6,
            solvable=True,
            cooperative=True,
            independent=False,
            chained={2: True, 3: False},
            interdependent={2: True, 3: False},
        ),
        expect(
            7,
            solvable=True,
            cooperative=True,
            independent=False,
            interdependent={2: True},
        ),
        *expect_for(
            range(8, 12),
            solvable=True,
            cooperative=True,
            independent=False,
            interdependent={2: False},
        ),
        *expect_for(
            range(12, 15),
            solvable=True,
            cooperative=False,
            independent=True,
            interdependent={2: False},
        ),
    ),
    description="Mutual help ends at t=8; cooperation ends when the detour opens at t=12.",
)

TWO_AGENT_MUTUAL_REVERSED = Layout(
    "two-agent-mutual-reversed",
    """
 .  . . S1 S0  .  . . .
L1E . .  .  .  @  @ @ .
 .  . @  .  . L0W . . .
 .  . .  .  .  .  . . .
 .  . .  X  X  .  . . .
""",
    (expect(6, solvable=True, cooperative=True, interdependent={2: True}),),
)

TEMPORAL_FLATTENING_COUNTEREXAMPLE = Layout(
    "temporal-flattening-counterexample",
    """
 @  @  L1S @ L3S  @   @
 @  S0 S1  @ S3  S2   @
L0E .   .  @  .   .  L2W
 @  .   X  @  X   .   @
 @  .   .  .  .   .   @
 @ L2E  X  @  X  L0W  @
""",
    (
        expect(
            20,
            cooperative=True,
            asymmetric=False,
            chained={2: True, 3: False},
            interdependent={2: True, 3: False},
        ),
    ),
    description="""
    This example shows that chronological order is important in the cooperation graph:
        - Agent 0 goes from left to right:
            - help(0, 1, t=1)
            - help(1, 0, t=4)
            - help(3, 0, t>=6)
        - Agent 2 goes from right to left:
            - help(2, 3, t=1)
            - help(3, 2, t=8)
            - help(1, 2, t>=10)
    In a flattened (time-agnostic) graph, the 4 vertices form a SCC while there exists actually
    no dependency at all between agent 0 and agent 2.

    Graph with nodes 0, 1, 2, 3 and the temporal edges enumerated above
        ┌----t=4----┐
        V           |
        0 ---t=1--> 1---┐
        ^               |
        └-----t>=6--┐   |
        ┌----t=10---|---┘
        V           |
        2 ---t=1--> 3
        ^           |
        └----t=8----┘

    What is checked here:
        - interdependent-2 is True (0 <-> 1, 2 <-> 3, 0 <-> 3)
        - interdependent-3 is False, because there exists no temporal cycle of 3 agents
        - there exists a chain of length 2 but no larger chain exists
        - there is no asymmetric help
    """,
)

THREE_AGENT_TEMPORAL_CYCLE = Layout(
    "three-agent-temporal-cycle",
    """
 @ L0S L2S L1S .
S0  .   .   .  X
S1  .   .   .  X
S2  .   .   .  X
""",
    (
        expect(
            15,
            solvable=True,
            chained={2: True, 3: True, 4: False},
            interdependent={2: True, 3: True, 4: False},
        ),
    ),
)

THREE_AGENT_WITH_TWO_AGENT_CYCLE = Layout(
    "three-agent-with-two-agent-cycle",
    """
 .  S1  S0 S2 @ @
 . L0E  .  .  @ @
 .  .  L1W .  . .
 @  .   .  @  . .
L2E .   .  .  . .
 @  @   .  X  X X
""",
    (
        expect(
            16,
            solvable=True,
            chained={2: True, 3: False},
            interdependent={2: True, 3: False},
        ),
    ),
)

PAPER_INTERDEPENDENT_3 = Layout(
    "paper-interdependent-3",
    """
 @   S0   @ L2S   S1 @
L0E  .    .   .   .  @
 @   .    @   X   .  @
 @   .    @   .   X L1W
 @   .    @   .   .  @
 @   .    .   X   S2 @
""",
    (
        expect(
            10,
            solvable=True,
            chained={2: True, 3: True},
            convergent={2: False},
            interdependent={2: False, 3: True, 4: False},
        ),
    ),
    description="This layout is presented in the paper as a canonical example of interdependent-3",
)

FOUR_AGENT_INTERDEPENDENT_4_CHAIN_6 = Layout(
    "four-agent-interdependent-4-chain-6",
    """
@   S0  S1 @   @
@   .   .  L1W @
L0E .   .  L1S @
@   X   .  .   @
@   L2S .  .   S2
@   X   .  @   @
S3  .   .  .   @
@   L3E .  .   @
@   @   .  .   L1W
@   @   X  X   @
""",
    (
        expect(
            14,
            solvable=True,
            chained={2: True, 3: True, 4: True, 5: True, 6: True, 7: False},
            interdependent={2: True, 3: True, 4: True, 5: False},
            convergent={2: True, 3: False},
            divergent={2: True, 3: True, 4: False},
        ),
    ),
)

EIGHT_AGENT_INTERDEPENDENT_8 = Layout(
    "eight-agent-interdependent-8",
    """
  .   .  .   .   .  .   .   . L1S  .
 L0E S0  X   .   .  .   .   . S1   .
  .   .  .  L5S  .  .   .   .  X   .
  .   .  .  S5   .  X  S4  L4W .   .
  .   .  .   X   .  .   .   .  .   .
  .   X  .   .   .  .   X   .  .   .
  X  S7  X  S6  L6W .  S3   X S2  L2W
  .  L7N .   .   .  .  L3N  .  .   .
""",
    (expect(1, interdependent={7: False, 8: True, 9: False}),),
    description="Each agent starts at the intersection of its own beam and its predecessor's "
    "blocked beam, forming the exact cycle 0 -> 1 -> ... -> 7 -> 0 at t=0.",
)


ALL_LAYOUTS = [
    LEVEL_1,
    LEVEL_2,
    LEVEL_3,
    LEVEL_4,
    LEVEL_5,
    LEVEL_6,
    BLOCKED_UNSOLVABLE,
    OPEN_TWO_AGENT,
    OPEN_TWO_AGENT_WIDE,
    ONE_WAY_DETOUR,
    SINGLE_LASER_ASYMMETRIC,
    DOUBLE_DISJOINT_ASYMMETRIC,
    CONVERGENT_2_TIGHT,
    DIVERGENT_2_TIGHT,
    DIVERGENT_2_WITH_DETOUR,
    CHAIN_4_WITH_MUTUAL,
    CHAIN_3_WITHOUT_CYCLE,
    PAPER_CHAIN_2,
    PAPER_CHAIN_2_NOT_INTERDEPENDENT_3,
    PAPER_CONVERGENT_2,
    PAPER_FULLY_COUPLED,
    PAPER_FULLY_COUPLED_LEGACY,
    TWO_AGENT_MUTUAL_COMPACT,
    TWO_AGENT_MUTUAL_WITH_DETOURS,
    TWO_AGENT_MUTUAL_REVERSED,
    TEMPORAL_FLATTENING_COUNTEREXAMPLE,
    THREE_AGENT_TEMPORAL_CYCLE,
    THREE_AGENT_WITH_TWO_AGENT_CYCLE,
    PAPER_INTERDEPENDENT_3,
    FOUR_AGENT_INTERDEPENDENT_4_CHAIN_6,
    EIGHT_AGENT_INTERDEPENDENT_8,
]

LAYOUTS_BY_NAME = {layout.name: layout for layout in ALL_LAYOUTS}


def scalar_cases_for(name: ScalarPropertyName):
    """Return every explicit expectation for a scalar property."""
    return [
        ScalarPropertyCase(layout, expectation.t_max, expected)
        for layout in ALL_LAYOUTS
        for expectation in layout.expectations
        if (expected := getattr(expectation.properties, name)) is not None
    ]


def chained_cases():
    """Return every explicit chain-length expectation."""
    return [
        ChainedCase(layout, expectation.t_max, expected, length)
        for layout in ALL_LAYOUTS
        for expectation in layout.expectations
        for length, expected in expectation.properties.chained.items()
    ]


def convergent_cases():
    """Return every explicit convergence-threshold expectation."""
    return [
        ConvergentCase(layout, expectation.t_max, expected, k)
        for layout in ALL_LAYOUTS
        for expectation in layout.expectations
        for k, expected in expectation.properties.convergent.items()
    ]


def divergent_cases():
    """Return every explicit divergence-threshold expectation."""
    return [
        DivergentCase(layout, expectation.t_max, expected, k)
        for layout in ALL_LAYOUTS
        for expectation in layout.expectations
        for k, expected in expectation.properties.divergent.items()
    ]


def interdependent_cases():
    """Return every explicit interdependence-order expectation."""
    return [
        InterdependentCase(layout, expectation.t_max, expected, order)
        for layout in ALL_LAYOUTS
        for expectation in layout.expectations
        for order, expected in expectation.properties.interdependent.items()
    ]
