from __future__ import annotations

from dataclasses import dataclass
from typing import Literal, NotRequired, TypedDict

from lle.generator.world_filter import WorldFilter


def _resolve_lasers(n_lasers: int | Literal["auto"], n_agents: int, wants_cooperation: bool) -> int:
    if n_lasers == "auto":
        if not wants_cooperation:
            return 0
        return max(1, n_agents - 1)
    return n_lasers


def _no_default_for(kind: str, arg_name: str):
    return ValueError(f"No default value for parameter {arg_name} with kind={kind!r}")


@dataclass
class GenerateArgs:
    kind: Literal["auto", "random", "constructive", "level6_style"]
    height: int
    width: int
    n_agents: int
    t_min: int
    n_lasers: int | Literal["auto"] = "auto"
    cooperation: bool | None = None
    mutual: bool | None = None
    filter: WorldFilter | None = None
    t_max: int | Literal["auto"] = "auto"
    n_walls: int | Literal["auto"] = "auto"
    max_attempts: int | None = None
    n_jobs: int | Literal["auto"] = "auto"

    def _resolve_size(self) -> tuple[int, int]:
        if (self.width is None and self.height is not None) or (self.height is None and self.width is not None):
            raise ValueError("Cannot infer the size of the grid. Either provide both `width` and `height` or none of them.")
        if self.width is None or self.height is None:
            if self.kind in ("random", "constructive"):
                return 5, 5
            elif self.kind == "level6_style":
                return 12, 13
            else:
                raise _no_default_for(self.kind, "size")
        return self.height, self.width

    def _resolve_n_agents(self, filter: WorldFilter):
        assert self.n_agents > 0
        if filter.cooperative and self.n_agents < 2:
            raise ValueError("n_agents must be at least 2 for cooperative worlds")
        return self.n_agents

    def _resolve_t_max(self) -> int:
        if self.t_max == "auto":
            return (self.width * self.height) // 2
        return self.t_max

    def _resolve_t_min(self) -> int:
        if self.t_min == "auto":
            return 0
        return self.t_min

    def _resolve_n_walls(self):
        if self.n_walls == "auto":
            return (self.width * self.height) // 10
        return self.n_walls

    def _resolve_n_jobs(self, n: int) -> int:
        if self.n_jobs == "auto":
            if n > 1:
                from multiprocessing import cpu_count

                return cpu_count() - 1
            return 1
        return self.n_jobs

    def resolve_filter(self):
        if self.filter is not None:
            if self.cooperation is not None or self.mutual is not None:
                raise ValueError(
                    "Cannot combine a WorldFilter with the 'cooperation' or 'mutual' keyword arguments. "
                    "Either pass a WorldFilter or use the keyword shortcuts, not both."
                )
            return self.filter
        return WorldFilter(cooperative=self.cooperation, mutual=self.mutual)

    def resolve_kind(self, filter: WorldFilter):
        if self.kind != "auto":
            return self.kind
        if filter.indepdenent:
            return "random"
        if filter.mutual:
            return "level6_style"
        return "constructive"

    def resolve(self, n: int = 1) -> _SanitizedArgs:
        filter = self.resolve_filter()
        kind = self.resolve_kind(filter)
        n_agents = self._resolve_n_agents(filter)
        t_max = self._resolve_t_max()
        n_walls = self._resolve_n_walls()
        n_lasers = _resolve_lasers(self.n_lasers, n_agents)

        n_jobs = self._resolve_n_jobs(n)
        return _SanitizedArgs(
            kind=self.kind,
            height=self.height,
            width=self.width,
            n_agents=n_agents,
            n_lasers=n_lasers,
            t_max=t_max,
            t_min=self.t_min,
            n_walls=n_walls,
            max_attempts=self.max_attempts,
            n_jobs=n_jobs,
            filter=filter,
        )


@dataclass
class _SanitizedArgs:
    kind: Literal["random", "constructive", "level6_style"]
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


class _CommonKWArgs(TypedDict, total=False):
    height: int
    width: int
    n_agents: int
    n_lasers: int | Literal["auto"]
    t_min: int
    t_max: int | Literal["auto"]
    n_walls: int | Literal["auto"]
    seed: int | None
    n_jobs: int | Literal["auto"]
    kind: Literal["auto", "level6_style", "random", "constructive"]


class _ConfigKW(_CommonKWArgs):
    cooperative: NotRequired[bool | None]
    mutual: NotRequired[bool | None]


class _ConfigFilter(_CommonKWArgs):
    filter: WorldFilter


class _ConfigMultiple(TypedDict):
    n: int
    quiet: NotRequired[bool]
    max_attempts: NotRequired[int]


class _ConfigMultipleKW(_ConfigMultiple, _ConfigKW): ...


class _ConfigMultipleFilter(_ConfigMultiple, _ConfigFilter): ...
