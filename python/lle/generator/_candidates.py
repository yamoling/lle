from __future__ import annotations

from dataclasses import dataclass

from lle.tiles import Direction


@dataclass(frozen=True)
class CandidateLayout:
    """ "Candidate layouts sampled by generators before world construction.

    A `CandidateLayout` stores the raw positions chosen by a generator before the
    layout is turned into a `World`.
    """

    agents: list[tuple[int, int]]
    exits: list[tuple[int, int]]
    walls: list[tuple[int, int]]
    lasers: list[tuple[int, tuple[int, int], Direction]]  # (owner, pos, dir)
