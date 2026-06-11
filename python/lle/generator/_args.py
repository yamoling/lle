"""Argument resolution for `lle.generate`.

`generate()` accepts a number of human-friendly arguments — several of which
accept ``"auto"`` or ``None`` and need to be turned into concrete values before
a generator can run. `GenerateArgs` holds the *raw* user arguments together with
their defaults (so the defaults live next to their types), and `resolve()`
turns them into a fully concrete `SanitizedArgs`.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal

from .world_filter import Cooperative, GeneratorKind, Independent, Mutual, Solvable, WorldFilter


def build_filter(filter: WorldFilter | None, cooperative: bool | None, mutual: bool | None) -> WorldFilter:
    """Turn the ``filter`` / ``cooperative`` / ``mutual`` shortcuts into a single `WorldFilter`.

    The shortcuts and an explicit ``filter`` are mutually exclusive. The
    contradictory ``cooperative=False, mutual=True`` is rejected here, at the
    boundary, since the `WorldFilter` hierarchy cannot represent it at all.
    """
    if filter is not None:
        if cooperative is not None or mutual is not None:
            raise ValueError(
                "Cannot combine a WorldFilter with the 'cooperative' or 'mutual' keyword arguments. "
                "Either pass a WorldFilter or use the keyword shortcuts, not both."
            )
        return filter
    if mutual and cooperative is False:
        raise ValueError("Mutual cooperation requires cooperation: 'cooperative=False, mutual=True' is contradictory.")
    if mutual:
        return Mutual()
    if cooperative:
        return Cooperative()
    if cooperative is False:
        return Independent()
    return Solvable()


@dataclass
class GenerateArgs:
    """Raw, user-facing generation arguments with their defaults.

    Fields that accept ``"auto"`` are resolved lazily in `resolve()` so that the
    resolution can depend on other (already concrete) fields, e.g. ``n_lasers``
    depends on whether the filter requires cooperation.
    """

    kind: Literal["auto", "random", "constructive", "level6_style"] = "auto"
    height: int = 10
    width: int = 10
    n_agents: int = 3
    n_lasers: int | Literal["auto"] = "auto"
    t_min: int = 0
    t_max: int | Literal["auto"] = "auto"
    n_walls: int | Literal["auto"] = "auto"
    max_attempts: int | None = None
    n_jobs: int | Literal["auto"] = 1
    filter: WorldFilter = field(default_factory=Solvable)

    def _resolve_kind(self) -> GeneratorKind:
        if self.kind != "auto":
            return self.kind
        return self.filter.default_kind

    def _resolve_t_max(self) -> int:
        if self.t_max == "auto":
            return (self.width * self.height) // 2
        return self.t_max

    def _resolve_n_walls(self) -> int:
        if self.n_walls == "auto":
            return (self.width * self.height) // 10
        return self.n_walls

    def _resolve_n_lasers(self, kind: GeneratorKind) -> int:
        if self.n_lasers != "auto":
            n_lasers = self.n_lasers
        # ``level6_style`` always needs at least one laser; cooperative worlds
        # need roughly one helper per other agent. Independent worlds need none.
        elif kind == "level6_style" or self.filter.requires_cooperation:
            n_lasers = min(self.n_agents, max(1, self.n_agents - 1))
        else:
            n_lasers = 0
        # Check that the number of lasers makes sense with the filter requirements
        if self.filter.requires_cooperation and n_lasers == 0:
            raise ValueError("Cooperative worlds are impossible with 0 laser.")
        if self.filter.requires_mutual_cooperation and n_lasers < 2:
            raise ValueError("Mutual cooperation are impossible with less than two lasers.")
        return n_lasers

    def _resolve_n_jobs(self, n: int) -> int:
        if self.n_jobs == "auto":
            if n > 1:
                from multiprocessing import cpu_count

                return max(1, cpu_count() - 1)
            return 1
        return self.n_jobs

    def resolve(self, n: int = 1) -> SanitizedArgs:
        if self.n_agents < 1:
            raise ValueError(f"n_agents must be >= 1. Got {self.n_agents}")
        if self.filter.requires_cooperation and self.n_agents < 2:
            raise ValueError("Cooperative worlds require at least 2 agents.")
        kind = self._resolve_kind()
        return SanitizedArgs(
            kind=kind,
            height=self.height,
            width=self.width,
            n_agents=self.n_agents,
            n_lasers=self._resolve_n_lasers(kind),
            t_max=self._resolve_t_max(),
            t_min=self.t_min,
            n_walls=self._resolve_n_walls(),
            max_attempts=self.max_attempts,
            n_jobs=self._resolve_n_jobs(n),
            filter=self.filter,
        )


@dataclass
class SanitizedArgs:
    """Fully concrete generation arguments, ready to instantiate a generator."""

    kind: GeneratorKind
    height: int
    width: int
    n_agents: int
    n_lasers: int
    filter: WorldFilter
    t_max: int
    t_min: int
    n_walls: int
    max_attempts: int | None
    n_jobs: int
