"""CandidateLayout: the shape sampled by generators before world-building."""

from __future__ import annotations

from dataclasses import dataclass

from lle.tiles import Direction


@dataclass(frozen=True)
class CandidateLayout:
    """Immutable layout snapshot sampled by a generator before world construction."""

    agents: list[tuple[int, int]]
    exits: list[tuple[int, int]]
    walls: list[tuple[int, int]]
    lasers: list[tuple[int, tuple[int, int], Direction]]  # (owner, pos, dir)
