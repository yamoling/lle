"""Composable behavioural predicates for generated worlds.

A :class:`Predicate` describes a world property computed by
:class:`lle.characterization.WorldCharacterizer`. Predicates are value objects:
they do not own a solver horizon. The generator wraps them in a
:class:`Constraint`, which adds the shared ``t_max`` evaluation horizon and
optional ``t_min`` shortest-solution threshold used when accepting or rejecting
candidate worlds.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Iterable, TypeAlias

from ..characterization.world_characterization import WorldCharacterizer
from ..world import World


@dataclass(frozen=True)
class WorldRequirements:
    """Minimum requirements for a world derived from predicates.

    For instance, a `Cooperative` predicate requires at least 2 agents,
    otherwise it can never be satisfied.

    These requirements are used for upfront validation and generator defaults.
    They are not a substitute for evaluating the predicate against a candidate
    world with `Constraint.is_satisfied_by`.
    """

    min_lasers: int = 0
    min_agents: int = 1

    def __post_init__(self):
        assert self.min_lasers >= 0
        assert self.min_agents >= 1

    @staticmethod
    def all(requirements: Iterable[WorldRequirements]) -> WorldRequirements:
        """Requirements for an ``AND``: every positive child must be possible."""
        rs = list(requirements)
        if not rs:
            return WorldRequirements()
        return WorldRequirements(
            min_lasers=max(r.min_lasers for r in rs),
            min_agents=max(r.min_agents for r in rs),
        )

    @staticmethod
    def any(requirements: Iterable[WorldRequirements]) -> WorldRequirements:
        """Requirements for an ``OR``: any one branch may satisfy the predicate."""
        rs = list(requirements)
        if not rs:
            return WorldRequirements()
        return WorldRequirements(
            min_lasers=min(r.min_lasers for r in rs),
            min_agents=min(r.min_agents for r in rs),
        )


class Predicate(ABC):
    """Boolean predicate over a solvable world's characterization."""

    @abstractmethod
    def holds(self, c: WorldCharacterizer) -> bool:
        """Return whether a solvable world characterized by ``c`` satisfies this predicate."""

    @property
    def requirements(self):
        """Static resource requirements implied by this predicate."""
        return WorldRequirements()

    @property
    def cost(self) -> int:
        """Static evaluation-cost estimate used to order short-circuit checks."""
        return 0

    def __and__(self, other: Predicate) -> Predicate:
        return And(self, other)

    def __or__(self, other: Predicate) -> Predicate:
        return Or(self, other)

    def __invert__(self) -> Predicate:
        return Not(self)

    def and_(self, other: Predicate) -> Predicate:
        return self & other

    def or_(self, other: Predicate) -> Predicate:
        return self | other

    def not_(self) -> Predicate:
        return ~self

    def and_not(self, other: Predicate) -> Predicate:
        return self & ~other


@dataclass(frozen=True)
class Solvable(Predicate):
    """Matches any solvable world.

    Solvability is checked by :class:`Constraint`, so this atom always holds
    once predicate evaluation begins.
    """

    def holds(self, c: WorldCharacterizer) -> bool:
        return c.is_solvable()


@dataclass(frozen=True)
class Independent(Predicate):
    """Matches worlds solvable without cooperation."""

    def holds(self, c: WorldCharacterizer) -> bool:
        return c.is_independent()

    @property
    def cost(self) -> int:
        return 1


@dataclass(frozen=True)
class Cooperative(Predicate):
    """Matches worlds that require at least one laser-blocking cooperation."""

    def holds(self, c: WorldCharacterizer) -> bool:
        return c.is_cooperative()

    @property
    def requirements(self) -> WorldRequirements:
        return WorldRequirements(min_lasers=1, min_agents=2)

    @property
    def cost(self) -> int:
        return 2


@dataclass(frozen=True)
class Asymmetric(Predicate):
    """Matches worlds that require asymmetric cooperation."""

    def holds(self, c: WorldCharacterizer) -> bool:
        return c.is_asymmetric()

    @property
    def requirements(self) -> WorldRequirements:
        return WorldRequirements(min_lasers=1, min_agents=2)

    @property
    def cost(self) -> int:
        return 3


@dataclass(frozen=True)
class Sequential(Predicate):
    """Matches worlds that require a sequential dependency of at least ``length`` help edges."""

    length: int = 2

    def __post_init__(self) -> None:
        if self.length < 2:
            raise ValueError(f"Sequence length must be >= 2, got {self.length}.")

    def holds(self, c: WorldCharacterizer) -> bool:
        return c.is_sequential(self.length)

    @property
    def requirements(self) -> WorldRequirements:
        return WorldRequirements(min_lasers=self.length, min_agents=2)

    @property
    def cost(self) -> int:
        return 10 + self.length


@dataclass(frozen=True)
class Convergent(Predicate):
    """Matches worlds requiring one beneficiary to receive help from ``k`` agents."""

    k: int

    def __post_init__(self) -> None:
        """Reject thresholds that cannot express convergence."""
        if self.k < 2:
            raise ValueError(f"Convergence requires at least 2 distinct helpers, got {self.k}.")

    def holds(self, c: WorldCharacterizer) -> bool:
        return c.is_convergent(self.k)

    @property
    def requirements(self) -> WorldRequirements:
        return WorldRequirements(min_lasers=self.k, min_agents=self.k + 1)

    @property
    def cost(self) -> int:
        return 20 + self.k


@dataclass(frozen=True)
class Divergent(Predicate):
    """Matches worlds requiring one helper to assist ``k`` distinct beneficiaries."""

    k: int = 2

    def __post_init__(self) -> None:
        """Reject thresholds that cannot express divergence."""
        if self.k < 2:
            raise ValueError(f"Divergence requires at least 2 distinct beneficiaries, got {self.k}.")

    def holds(self, c: WorldCharacterizer) -> bool:
        return c.is_divergent(self.k)

    @property
    def requirements(self) -> WorldRequirements:
        return WorldRequirements(min_lasers=1, min_agents=self.k + 1)

    @property
    def cost(self) -> int:
        return 20 + self.k


@dataclass(frozen=True)
class Interdependent(Predicate):
    """Matches worlds that require a temporal dependency cycle of at least ``order`` agents."""

    order: int = 2

    def __post_init__(self) -> None:
        if self.order < 2:
            raise ValueError(f"Dependency order must be >= 2, got {self.order}.")

    def holds(self, c: WorldCharacterizer) -> bool:
        return c.is_interdependent(self.order)

    @property
    def requirements(self) -> WorldRequirements:
        return WorldRequirements(min_lasers=self.order, min_agents=self.order)

    @property
    def cost(self) -> int:
        return 20 + self.order


@dataclass(frozen=True, init=False)
class And(Predicate):
    children: tuple[Predicate, ...]

    def __init__(self, *children: Predicate):
        object.__setattr__(self, "children", _flatten(children, And))

    def holds(self, c: WorldCharacterizer) -> bool:
        return all(p.holds(c) for p in self._ordered())

    @property
    def requirements(self) -> WorldRequirements:
        return WorldRequirements.all(p.requirements for p in self.children)

    @property
    def cost(self) -> int:
        return sum(p.cost for p in self.children)

    def _ordered(self) -> tuple[Predicate, ...]:
        return tuple(sorted(self.children, key=lambda p: p.cost))


@dataclass(frozen=True, init=False)
class Or(Predicate):
    children: tuple[Predicate, ...]

    def __init__(self, *children: Predicate):
        object.__setattr__(self, "children", _flatten(children, Or))

    def holds(self, c: WorldCharacterizer) -> bool:
        return any(p.holds(c) for p in self._ordered())

    @property
    def requirements(self) -> WorldRequirements:
        return WorldRequirements.any(p.requirements for p in self.children)

    @property
    def cost(self) -> int:
        return sum(p.cost for p in self.children)

    def _ordered(self) -> tuple[Predicate, ...]:
        return tuple(sorted(self.children, key=lambda p: p.cost))


@dataclass(frozen=True)
class Not(Predicate):
    inner: Predicate

    def holds(self, c: WorldCharacterizer) -> bool:
        return not self.inner.holds(c)

    @property
    def requirements(self) -> WorldRequirements:
        return WorldRequirements()

    @property
    def cost(self) -> int:
        return self.inner.cost


@dataclass(frozen=True)
class Constraint:
    """A predicate plus the solver horizon used to evaluate generated worlds."""

    t_max: int
    predicate: Predicate = field(default_factory=Solvable)
    min_solution_length: int | None = None

    @property
    def requirements(self):
        return self.predicate.requirements

    def is_satisfied_by(self, world: World):
        """Return whether `world` satisfies this generator constraint."""
        c = WorldCharacterizer(world, self.t_max)
        if self.min_solution_length is not None:
            if c.shortest_path is None:
                return False
            if len(c.shortest_path) < self.min_solution_length:
                return False
        return self.predicate.holds(c)


# Backwards-compatible name for type annotations/imports during the transition.
WorldFilter: TypeAlias = Constraint


def _flatten(children: Iterable[Predicate], cls: type[And] | type[Or]) -> tuple[Predicate, ...]:
    flattened: list[Predicate] = []
    for child in children:
        if not isinstance(child, Predicate):
            raise TypeError(f"Expected Predicate, got {type(child).__name__}.")
        if isinstance(child, cls):
            flattened.extend(child.children)
        else:
            flattened.append(child)
    return tuple(flattened)
