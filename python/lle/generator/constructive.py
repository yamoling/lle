from __future__ import annotations

from lle.tiles import Direction

from ._candidates import CandidateLayout
from ._geometry import beam_tiles, points_out_immediately
from .random import RandomGenerator


class ConstructiveGenerator(RandomGenerator):
    """
    Reserves one disjoint lane per agent so a joint solution exists by
    construction, then places walls and lasers only outside those lanes.
    SAT is still used as a final verifier.
    """

    def _make_candidate_layout(self) -> CandidateLayout:
        if self.world_filter.requires_cooperation:
            layout = self._make_cooperative_candidate_layout()
            if layout is None:
                return super()._make_candidate_layout()
            return layout
        layout = self._make_constructive_candidate_layout()
        if layout is None:
            return super()._make_candidate_layout()
        return layout

    # Number of independent lane samples tried before falling back to the
    # parent's random layout in the cooperative path.
    _LANE_SAMPLE_ATTEMPTS: int = 8

    def _make_cooperative_candidate_layout(self) -> CandidateLayout | None:
        """Random non-contiguous lanes + random rotation + ``n_lasers``
        distinct-colour structural lasers, each perpendicular to the lane
        band so its beam crosses every lane.

        With ``n_lasers >= 2`` each laser's beam can only be safely
        truncated by the unique agent of its colour, so cooperation
        involves ``n_lasers`` helpers acting on their own beams in turn
        (mutual / distributed profile) instead of a single helper on one
        structural beam (asymmetric profile).
        """
        if self.agents < 2 or self.n_lasers < 1:
            return None
        feasible: list[str] = []
        # Need at least one non-lane axis slot for a structural laser and
        # perp axis >= 3 so the laser perp coord can land in the interior
        # (avoiding agent/exit columns).
        if self.rows >= self.agents + 1 and self.cols >= 3:
            feasible.append("horizontal")
        if self.cols >= self.agents + 1 and self.rows >= 3:
            feasible.append("vertical")
        if not feasible:
            return None
        # Pick orientation uniformly per call so both rotations occur
        # across attempts (rather than greedily preferring one whenever it
        # succeeds).
        orientation = self._rng.choice(feasible)
        for _ in range(self._LANE_SAMPLE_ATTEMPTS):
            layout = self._build_cooperative_lane_layout(orientation)
            if layout is not None:
                return layout
        # Fall back to the other orientation if the chosen one repeatedly
        # fails (e.g., lane sample kept landing at both grid edges).
        for other in feasible:
            if other == orientation:
                continue
            for _ in range(self._LANE_SAMPLE_ATTEMPTS):
                layout = self._build_cooperative_lane_layout(other)
                if layout is not None:
                    return layout
        return None

    def _build_cooperative_lane_layout(self, orientation: str) -> CandidateLayout | None:
        """One attempt: random non-contiguous lanes + random rotation +
        ``n_lasers`` distinct-colour structural lasers, each crossing the
        whole lane band. Distinct perpendicular columns guarantee the
        parallel beams are on disjoint cells. The full unblocked beam
        path of every laser is reserved so walls cannot clip it. Returns
        ``None`` if the lane sample leaves no axis slot on either side of
        the band, or if there are not enough free cells for ``n_walls``.
        """
        if orientation == "horizontal":
            lane_axis_size, perp_axis_size = self.rows, self.cols
        else:
            lane_axis_size, perp_axis_size = self.cols, self.rows

        lane_ids = sorted(self._rng.sample(range(lane_axis_size), self.agents))
        lane_set = set(lane_ids)
        non_lane = [i for i in range(lane_axis_size) if i not in lane_set]
        if not non_lane:
            return None

        min_lane, max_lane = lane_ids[0], lane_ids[-1]
        before_band = [i for i in non_lane if i < min_lane]
        after_band = [i for i in non_lane if i > max_lane]
        axis_dir_options: list[tuple[int, Direction]] = []
        if orientation == "horizontal":
            axis_dir_options.extend((r, Direction.SOUTH) for r in before_band)
            axis_dir_options.extend((r, Direction.NORTH) for r in after_band)
        else:
            axis_dir_options.extend((c, Direction.EAST) for c in before_band)
            axis_dir_options.extend((c, Direction.WEST) for c in after_band)
        if not axis_dir_options:
            return None

        valid_perps = list(range(1, perp_axis_size - 1))
        if len(valid_perps) < self.n_lasers:
            return None
        chosen_perps = self._rng.sample(valid_perps, self.n_lasers)

        # One distinct-colour structural laser per perpendicular column.
        laser_placements: list[tuple[int, tuple[int, int], Direction]] = []
        for colour, laser_perp in enumerate(chosen_perps):
            laser_axis, direction = self._rng.choice(axis_dir_options)
            if orientation == "horizontal":
                source = (laser_axis, laser_perp)
            else:
                source = (laser_perp, laser_axis)
            laser_placements.append((colour, source, direction))

        # Random rotation: flip agent / exit edges so all 4 rotations
        # (agents on left / right / top / bottom) are equally likely.
        flip = self._rng.random() < 0.5
        if orientation == "horizontal":
            agent_col = self.cols - 1 if flip else 0
            exit_col = 0 if flip else self.cols - 1
            agents = [(row, agent_col) for row in lane_ids]
            exits = [(row, exit_col) for row in lane_ids]
            reserved = {(row, col) for row in lane_ids for col in range(self.cols)}
        else:
            agent_row = self.rows - 1 if flip else 0
            exit_row = 0 if flip else self.rows - 1
            agents = [(agent_row, col) for col in lane_ids]
            exits = [(exit_row, col) for col in lane_ids]
            reserved = {(row, col) for col in lane_ids for row in range(self.rows)}
        reserved.update(agents)
        reserved.update(exits)

        # Reserve each laser's source cell and full unblocked beam path
        # so walls cannot clip the beam between non-adjacent lanes.
        for _colour, source, direction in laser_placements:
            reserved.add(source)
            path = beam_tiles(
                source,
                direction,
                walls=set(),
                lasers=set(),
                rows=self.rows,
                cols=self.cols,
            )
            reserved.update(path)

        free_positions = [(row, col) for row in range(self.rows) for col in range(self.cols) if (row, col) not in reserved]
        if len(free_positions) < self.n_walls:
            return None
        # Shuffle so the wall set is a random subset of the free cells,
        # not the first ``n_walls`` in row-major order.
        self._rng.shuffle(free_positions)
        walls = free_positions[: self.n_walls]

        return CandidateLayout(
            agents=agents,
            exits=exits,
            walls=walls,
            lasers=laser_placements,
        )

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
            if free_cells < self.n_walls + self.n_lasers:
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

        free_positions = [(row, col) for row in range(self.rows) for col in range(self.cols) if (row, col) not in reserved]
        if len(free_positions) < self.n_walls + self.n_lasers:
            return None
        self._rng.shuffle(free_positions)
        walls = free_positions[: self.n_walls]
        laser_pool = free_positions[self.n_walls :]

        lasers = self._place_safe_lasers(
            reserved=reserved,
            wall_positions=walls,
            candidate_positions=laser_pool,
        )
        if lasers is None:
            return None
        return CandidateLayout(agents=agents, exits=exits, walls=walls, lasers=lasers)

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
                tiles = beam_tiles(pos, direction, walls, used_sources, self.rows, self.cols)
                if not tiles:
                    continue
                if any(tile in reserved for tile in tiles):
                    continue
                candidates.append((pos, direction, tiles))
        self._rng.shuffle(candidates)
        for pos, direction, tiles in candidates:
            if len(lasers) >= self.n_lasers:
                break
            if pos in used_sources:
                continue
            if any(existing_pos in tiles for _, existing_pos, _ in lasers):
                continue
            if any(tile in reserved for tile in tiles):
                continue
            lasers.append((len(lasers), pos, direction))
            used_sources.add(pos)
        if len(lasers) != self.n_lasers:
            return None
        return lasers
