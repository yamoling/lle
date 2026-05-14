"""Random sampling generator (internal)."""
from __future__ import annotations

from lle.tiles import Direction

from ._base import _BaseGenerator, _LayoutRetry
from ._candidates import CandidateLayout
from ._geometry import beam_tiles, points_out_immediately


class _RandomGenerator(_BaseGenerator):
    def _sample_unique_positions(self, k: int) -> list[tuple[int, int]]:
        all_pos = [(r, c) for r in range(self.rows) for c in range(self.cols)]
        return self._rng.sample(all_pos, k)

    def _random_direction(self) -> Direction:
        return self._rng.choice(
            [Direction.NORTH, Direction.SOUTH, Direction.EAST, Direction.WEST]
        )

    def _make_candidate_layout(self) -> CandidateLayout:
        total = self.agents + self.agents + self.num_walls + self.lasers
        chosen = self._sample_unique_positions(total)
        i = 0
        agents = chosen[i : i + self.agents]
        i += self.agents
        exits = chosen[i : i + self.agents]
        i += self.agents
        walls = chosen[i : i + self.num_walls]
        i += self.num_walls
        laser_pos = chosen[i : i + self.lasers]
        lasers = [
            (k, pos, self._random_direction()) for k, pos in enumerate(laser_pos)
        ]
        layout = CandidateLayout(
            agents=agents, exits=exits, walls=walls, lasers=lasers
        )
        if not self._geometry_ok(layout):
            raise _LayoutRetry()
        return layout

    def _geometry_ok(self, layout: CandidateLayout) -> bool:
        wall_set = set(layout.walls)
        laser_set = {pos for _, pos, _ in layout.lasers}
        exit_set = set(layout.exits)
        all_beam: set[tuple[int, int]] = set()
        for _owner, src, direction in layout.lasers:
            if points_out_immediately(src, direction, self.rows, self.cols):
                return False
            tiles = beam_tiles(
                src, direction, wall_set, laser_set, self.rows, self.cols
            )
            if not tiles:
                return False
            all_beam.update(tiles)
        if exit_set & all_beam:
            return False
        return True
