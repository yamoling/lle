"""Behavioural constraints applied to generated worlds.

A `WorldFilter` decides whether a generated `World` should be kept. The filters
form a small hierarchy rather than a bag of independent boolean flags so
that *contradictory* constraints are impossible to express in the first place:
there is no class that is simultaneously `Independent` and `Mutual`, because
`Mutual` is a subclass of `Cooperative` and `Independent` is its sibling. This
removes the old ``cooperative=False, mutual=True`` foot-gun by construction.

The taxonomy mirrors the cooperation lattice computed by `WorldCharacterizer`:

    WorldFilter (any solvable world)
    ├── Independent          – solvable without any cooperation
    └── Cooperative          – cooperation is required
        └── Mutual           – every agent both helps and is helped
        (future: FullyCoupled, Chained, Distributed …)

Every filter is a frozen dataclass so it pickles cleanly into worker processes.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Literal

from ..characterization.world_characterization import NotSolvableError, WorldCharacterizer
from ..world import World

GeneratorKind = Literal["random", "constructive", "level6_style"]
"""The procedural generators a filter can recommend when ``kind='auto'``."""


@dataclass(frozen=True)
class WorldFilter(ABC):
    """Base filter: matches every *solvable* world and imposes no further constraint.

    Concrete behavioural constraints are expressed by the subclasses
    `Independent`, `Cooperative` and `Mutual`. Use those directly, e.g.
    ``generate(filter=Mutual())``.
    """

    t_max: int | None = None
    """Override the ``t_max`` used for characterisation. ``None`` reuses the generator's own ``t_max``."""

    @abstractmethod
    def _matches(self, c: WorldCharacterizer) -> bool:
        """Whether a *solvable* world, characterised by ``c``, matches this filter."""

    def is_satisfied_by(self, world: World, default_t_max: int) -> bool:
        """Return whether ``world`` is solvable and matches this filter.

        ``default_t_max`` is the generator's horizon; it is used unless the
        filter carries its own ``t_max`` override.
        """
        effective_t_max = default_t_max if self.t_max is None else self.t_max
        c = WorldCharacterizer(world, effective_t_max)
        return self._matches(c)

    @property
    def requires_cooperation(self) -> bool:
        """Whether worlds matching this filter necessarily require cooperation."""
        return False

    @property
    def requires_mutual_cooperation(self) -> bool:
        return False

    @property
    def default_kind(self) -> GeneratorKind:
        """The generator strategy that best fits this filter when ``kind='auto'``."""
        return "constructive"


@dataclass(frozen=True)
class Solvable(WorldFilter):
    """Matches any solvable world. This is the default when no constraint is given."""

    def _matches(self, c: WorldCharacterizer) -> bool:
        return c.is_solvable


@dataclass(frozen=True)
class Independent(WorldFilter):
    """Matches worlds that are solvable *without* cooperation (no laser blocking required)."""

    def _matches(self, c: WorldCharacterizer) -> bool:
        try:
            return not c.is_cooperative
        except NotSolvableError:
            return False

    @property
    def default_kind(self) -> GeneratorKind:
        return "random"


@dataclass(frozen=True)
class Cooperative(WorldFilter):
    """Matches worlds that *require* cooperation: no independent plan exists within ``t_max``."""

    def _matches(self, c: WorldCharacterizer) -> bool:
        try:
            return c.is_cooperative
        except NotSolvableError:
            return False

    @property
    def requires_cooperation(self) -> bool:
        return True


@dataclass(frozen=True)
class Mutual(Cooperative):
    """Matches worlds that require *mutual* cooperation: every agent both helps and is helped.

    Mutual cooperation implies cooperation, hence this is a refinement of `Cooperative`.
    """

    def _matches(self, c: WorldCharacterizer) -> bool:
        # ``is_mutual`` already entails ``is_cooperative``.
        try:
            return c.is_mutual
        except NotSolvableError:
            return False

    @property
    def default_kind(self) -> GeneratorKind:
        return "level6_style"

    @property
    def requires_mutual_cooperation(self) -> bool:
        return True
