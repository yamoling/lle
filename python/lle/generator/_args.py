from __future__ import annotations

from dataclasses import dataclass
from multiprocessing import cpu_count
from typing import TYPE_CHECKING, Literal

from ..solver.cooperation_level import CooperationLevel
from .generator import CooperationSpec

if TYPE_CHECKING:
    from lle.generator import LooseCooperationSpec


def _resolve_lasers(n_lasers: int | Literal["auto"], n_agents: int, is_cooperative: bool) -> int:
    if n_lasers == "auto":
        if not is_cooperative:
            return 0
        return max(1, n_agents - 1)
    return n_lasers


def _no_default_for(kind: str, arg_name: str):
    return ValueError(f"No default value for parameter {arg_name} with kind={kind!r}")


@dataclass
class GenerateArgs:
    kind: Literal["random", "constructive", "level6_style"]
    height: int | None = None
    width: int | None = None
    n_agents: int | None = None
    n_lasers: int | Literal["auto"] = "auto"
    cooperation: LooseCooperationSpec | None = None
    t_max: int | Literal["auto"] = "auto"
    t_min: int | Literal["auto"] = "auto"
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

    def _resolve_n_agents(self):
        if self.n_agents is None:
            if self.kind in ("random", "constructive"):
                return 2
            elif self.kind == "level6_style":
                return 4
            else:
                raise _no_default_for(self.kind, "n_agents")
        return self.n_agents

    def _resolve_cooperation_requirement(self) -> CooperationSpec | None:
        match self.cooperation:
            case None:
                if self.kind in ("random", "constructive"):
                    return None
                if self.kind == "level6_style":
                    return "exactly", CooperationLevel.MUTUAL
                raise _no_default_for(self.kind, "cooperation")
            case True:
                return "at-least", CooperationLevel.ASYMMETRIC
            case False:
                return "exactly", CooperationLevel.INDEPENDENT
            case CooperationLevel(level) | str(level):
                return "exactly", CooperationLevel(level)
            case (op, str(level)):
                return op, CooperationLevel(level)
            case (op, CooperationLevel(level)):
                return op, level
        raise ValueError(f"Invalid cooperation specification: {self.cooperation!r}")

    def _resolve_t_max(self, width: int, height: int) -> int:
        if self.t_max == "auto":
            if self.kind == "level6_style":
                return 21
            else:
                return width * height // 2
        return self.t_max

    def _resolve_t_min(self) -> int:
        # "auto" means no lower bound on the solution length.
        if self.t_min == "auto":
            return 0
        return self.t_min

    def _resolve_n_walls(self, width: int, height: int):
        if self.n_walls == "auto":
            return width * height // 10
        return self.n_walls

    def _resolve_n_jobs(self, n: int) -> int:
        if self.n_jobs == "auto":
            if n > 1:
                return cpu_count() - 1
            return 1
        return self.n_jobs

    def resolve(self, n: int = 1) -> _SanitizedArgs:
        height, width = self._resolve_size()
        n_agents = self._resolve_n_agents()
        cooperation = self._resolve_cooperation_requirement()
        is_cooperative = cooperation is not None and cooperation[1].is_cooperative
        if self.kind == "level6_style" and not is_cooperative:
            raise ValueError("Levels in the style of level 6 must be cooperative.")
        t_max = self._resolve_t_max(width, height)
        t_min = self._resolve_t_min()
        n_lasers = _resolve_lasers(self.n_lasers, n_agents, is_cooperative)
        n_walls = self._resolve_n_walls(width, height)
        n_jobs = self._resolve_n_jobs(n)
        return _SanitizedArgs(
            kind=self.kind,
            height=height,
            width=width,
            n_agents=n_agents,
            n_lasers=n_lasers,
            cooperation=cooperation,
            t_max=t_max,
            t_min=t_min,
            n_walls=n_walls,
            max_attempts=self.max_attempts,
            n_jobs=n_jobs,
        )


@dataclass
class _SanitizedArgs:
    kind: Literal["random", "constructive", "level6_style"]
    height: int
    width: int
    n_agents: int
    n_lasers: int
    cooperation: CooperationSpec | None
    t_max: int
    t_min: int
    n_walls: int
    max_attempts: int | None
    n_jobs: int
