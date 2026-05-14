"""Constructive generator: reserves one lane per agent for a constructive solvability proof."""
from __future__ import annotations

from lle.tiles import Direction

from ._candidates import CandidateLayout
from ._geometry import beam_tiles, points_out_immediately
from ._random import _RandomGenerator


class _ConstructiveGenerator(_RandomGenerator):
    """
    Reserves one disjoint lane per agent so a joint solution exists by
    construction, then places walls and lasers only outside those lanes.
    SAT is still used as a final verifier.
    """

    def _make_candidate_layout(self) -> CandidateLayout:
        if self.cooperative:
            layout = self._make_cooperative_candidate_layout()
            if layout is None:
                return super()._make_candidate_layout()
            return layout
        layout = self._make_constructive_candidate_layout()
        if layout is None:
            return super()._make_candidate_layout()
        return layout

    def _make_cooperative_candidate_layout(self) -> CandidateLayout | None:
        """Place a structural same-colour laser across all agent lanes so
        cooperation is required by construction (standard SAT solvable
        because agent 0 can briefly block its own beam; strict-laser UNSAT
        because agent 0 cannot stay inside its own beam)."""
        if self.agents < 2 or self.rows < self.agents + 1 or self.cols < 4:
            return None
        if self.lasers < 1:
            return None

        lane_rows = list(range(1, self.agents + 1))
        beam_col = self._rng.randint(1, self.cols - 2)

        agents = [(row, 0) for row in lane_rows]
        exits = [(row, self.cols - 1) for row in lane_rows]

        reserved = set(agents) | set(exits)
        for row in lane_rows:
            for col in range(self.cols):
                reserved.add((row, col))
        structural_source = (0, beam_col)
        reserved.add(structural_source)

        structural_laser = (0, structural_source, Direction.SOUTH)
        lasers = [structural_laser]

        free_cells = [
            (row, col)
            for row in range(self.rows)
            for col in range(self.cols)
            if (row, col) not in reserved
        ]

        extras_needed = self.lasers - len(lasers)
        if extras_needed > 0:
            extras = self._place_extra_lasers_outside(
                reserved=reserved,
                candidate_positions=list(free_cells),
                existing_sources={structural_source},
                count=extras_needed,
            )
            if extras is None or len(extras) < extras_needed:
                return None
            lasers.extend(extras)

        used_laser_positions = {pos for _, pos, _ in lasers}
        walls = [cell for cell in free_cells if cell not in used_laser_positions]

        if self.num_walls > len(walls):
            return None
        walls = walls[: self.num_walls]

        return CandidateLayout(
            agents=agents, exits=exits, walls=walls, lasers=lasers
        )

    def _place_extra_lasers_outside(
        self,
        reserved: set[tuple[int, int]],
        candidate_positions: list[tuple[int, int]],
        existing_sources: set[tuple[int, int]],
        count: int,
    ) -> list[tuple[int, tuple[int, int], Direction]] | None:
        used_sources = set(existing_sources)
        placed: list[tuple[int, tuple[int, int], Direction]] = []

        candidates = []
        for pos in candidate_positions:
            for direction in [
                Direction.NORTH,
                Direction.SOUTH,
                Direction.EAST,
                Direction.WEST,
            ]:
                if points_out_immediately(pos, direction, self.rows, self.cols):
                    continue
                tiles = beam_tiles(
                    pos, direction, set(), used_sources, self.rows, self.cols
                )
                if not tiles:
                    continue
                if any(tile in reserved for tile in tiles):
                    continue
                candidates.append((pos, direction, tiles))

        self._rng.shuffle(candidates)

        for pos, direction, tiles in candidates:
            if len(placed) >= count:
                break
            if pos in used_sources:
                continue
            if any(existing in tiles for existing in used_sources):
                continue
            placed.append((1 + len(placed), pos, direction))
            used_sources.add(pos)

        return placed

    def _make_constructive_candidate_layout(self) -> CandidateLayout | None:
        orientations = []
        if self.rows >= self.agents:
            orientations.append(("horizontal", self.area - self.agents * self.cols))
        if self.cols >= self.agents:
            orientations.append(("vertical", self.area - self.agents * self.rows))
        if not orientations:
            return None
        orientations.sort(key=lambda item: item[1], reverse=True)
        for orientation, free_cells in orientations:
            if free_cells < self.num_walls + self.lasers:
                continue
            layout = self._build_lane_layout(orientation)
            if layout is not None:
                return layout
        return None

    def _build_lane_layout(self, orientation: str) -> CandidateLayout | None:
        if orientation == "horizontal":
            lane_ids = sorted(self._rng.sample(range(self.rows), self.agents))
            agents = [(row, 0) for row in lane_ids]
            exits = [(row, self.cols - 1) for row in lane_ids]
            reserved = {(row, col) for row in lane_ids for col in range(self.cols)}
        else:
            lane_ids = sorted(self._rng.sample(range(self.cols), self.agents))
            agents = [(0, col) for col in lane_ids]
            exits = [(self.rows - 1, col) for col in lane_ids]
            reserved = {(row, col) for col in lane_ids for row in range(self.rows)}

        free_positions = [
            (row, col)
            for row in range(self.rows)
            for col in range(self.cols)
            if (row, col) not in reserved
        ]
        if len(free_positions) < self.num_walls + self.lasers:
            return None
        self._rng.shuffle(free_positions)
        walls = free_positions[: self.num_walls]
        laser_pool = free_positions[self.num_walls :]

        lasers = self._place_safe_lasers(
            reserved=reserved,
            wall_positions=walls,
            candidate_positions=laser_pool,
        )
        if lasers is None:
            return None
        return CandidateLayout(
            agents=agents, exits=exits, walls=walls, lasers=lasers
        )

    def _place_safe_lasers(self, reserved, wall_positions, candidate_positions):
        walls = set(wall_positions)
        used_sources: set[tuple[int, int]] = set()
        lasers: list[tuple[int, tuple[int, int], Direction]] = []
        candidates = []
        for pos in candidate_positions:
            for direction in (
                Direction.NORTH,
                Direction.SOUTH,
                Direction.EAST,
                Direction.WEST,
            ):
                if points_out_immediately(pos, direction, self.rows, self.cols):
                    continue
                tiles = beam_tiles(
                    pos, direction, walls, used_sources, self.rows, self.cols
                )
                if not tiles:
                    continue
                if any(tile in reserved for tile in tiles):
                    continue
                candidates.append((pos, direction, tiles))
        self._rng.shuffle(candidates)
        for pos, direction, tiles in candidates:
            if len(lasers) >= self.lasers:
                break
            if pos in used_sources:
                continue
            if any(existing_pos in tiles for _, existing_pos, _ in lasers):
                continue
            if any(tile in reserved for tile in tiles):
                continue
            lasers.append((len(lasers), pos, direction))
            used_sources.add(pos)
        if len(lasers) != self.lasers:
            return None
        return lasers
