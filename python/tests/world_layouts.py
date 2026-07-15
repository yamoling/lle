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
    distributed: dict[int, bool] = field(default_factory=dict)
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
        """Build a fresh world from the shared layout definition.

        @ai-generated
        """
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
class DistributedCase:
    layout: Layout
    t_max: int
    expected: bool
    order: int

    @property
    def id(self):
        return f"{self.layout.name}-t{self.t_max}-{self.order}"


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
    distributed: dict[int, bool] | None = None,
    chained: dict[int, bool] | None = None,
    interdependent: dict[int, bool] | None = None,
) -> Expectation:
    """Declare the properties expected from one layout at one horizon.

    @ai-generated
    """
    return Expectation(
        t_max,
        ExpectedProperties(
            solvable=solvable,
            cooperative=cooperative,
            independent=independent,
            asymmetric=asymmetric,
            fully_coupled=fully_coupled,
            distributed={} if distributed is None else distributed,
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
    distributed: dict[int, bool] | None = None,
    chained: dict[int, bool] | None = None,
    interdependent: dict[int, bool] | None = None,
) -> tuple[Expectation, ...]:
    """Declare the same property matrix for several horizons.

    @ai-generated
    """
    return tuple(
        expect(
            t_max,
            solvable=solvable,
            cooperative=cooperative,
            independent=independent,
            asymmetric=asymmetric,
            fully_coupled=fully_coupled,
            distributed=distributed,
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
        *expect_for(range(10, 20), cooperative=True, independent=False, asymmetric=True),
    ),
)

LEVEL_4 = Layout(
    "level-4",
    4,
    (expect(10, cooperative=True, independent=False),),
)

LEVEL_5 = Layout(
    "level-5",
    5,
    (
        expect(19, cooperative=True, independent=False),
        expect(21, asymmetric=True),
        expect(25, asymmetric=False),
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
                4: False,
                5: False,
                6: False,
                7: False,
                8: False,
                9: False,
            },
            interdependent={2: True},
        ),
    ),
)


# Solvability and independent movement

BLOCKED_UNSOLVABLE = Layout(
    "blocked-unsolvable",
    "S0 @ X",
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


# One-way and asymmetric cooperation

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
    (expect(6, asymmetric=True),),
)


# Chained cooperation

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
            distributed={2: False},
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
            distributed={2: False},
            interdependent={2: False, 3: False},
        ),
    ),
)


# Distributed and fully coupled cooperation

PAPER_DISTRIBUTED_2 = Layout(
    "paper-distributed-2",
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
            distributed={2: True, 3: False},
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
            interdependent={2: True, 3: True},
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
            interdependent={2: True, 3: True},
        ),
    ),
)


# Mutual and interdependent cooperation

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
            independent=False,
            asymmetric=False,
            chained={2: True, 3: False},
            interdependent={2: True, 3: False},
        ),
    ),
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
            distributed={2: False},
            interdependent={2: False, 3: True},
        ),
    ),
    description="This layout is presented in the paper as a canonical example of interdependent-3",
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
    CHAIN_4_WITH_MUTUAL,
    CHAIN_3_WITHOUT_CYCLE,
    PAPER_CHAIN_2,
    PAPER_CHAIN_2_NOT_INTERDEPENDENT_3,
    PAPER_DISTRIBUTED_2,
    PAPER_FULLY_COUPLED,
    PAPER_FULLY_COUPLED_LEGACY,
    TWO_AGENT_MUTUAL_COMPACT,
    TWO_AGENT_MUTUAL_WITH_DETOURS,
    TWO_AGENT_MUTUAL_REVERSED,
    TEMPORAL_FLATTENING_COUNTEREXAMPLE,
    THREE_AGENT_TEMPORAL_CYCLE,
    THREE_AGENT_WITH_TWO_AGENT_CYCLE,
    PAPER_INTERDEPENDENT_3,
]

LAYOUTS_BY_NAME = {layout.name: layout for layout in ALL_LAYOUTS}


def scalar_cases_for(name: ScalarPropertyName):
    """Return every explicit expectation for a scalar property."""
    return tuple(
        ScalarPropertyCase(layout, expectation.t_max, expected)
        for layout in ALL_LAYOUTS
        for expectation in layout.expectations
        if (expected := getattr(expectation.properties, name)) is not None
    )


def chained_cases() -> tuple[ChainedCase, ...]:
    """Return every explicit chain-length expectation."""
    return tuple(
        ChainedCase(layout, expectation.t_max, expected, length)
        for layout in ALL_LAYOUTS
        for expectation in layout.expectations
        for length, expected in expectation.properties.chained.items()
    )


def distributed_cases() -> tuple[DistributedCase, ...]:
    """Return every explicit distributed-order expectation.

    @ai-generated
    """
    return tuple(
        DistributedCase(layout, expectation.t_max, expected, order)
        for layout in ALL_LAYOUTS
        for expectation in layout.expectations
        for order, expected in expectation.properties.distributed.items()
    )


def interdependent_cases() -> tuple[InterdependentCase, ...]:
    """Return every explicit interdependence-order expectation.

    @ai-generated
    """
    return tuple(
        InterdependentCase(layout, expectation.t_max, expected, order)
        for layout in ALL_LAYOUTS
        for expectation in layout.expectations
        for order, expected in expectation.properties.interdependent.items()
    )
