"""
Constructive cooperative generator with clustered starts and exits on
opposing sides of the grid (LLE Level 6 inspired).

Cooperation is intrinsic to this kind: the strict-laser UNSAT check runs
unconditionally (via cooperative=True forced in __init__).
"""

from __future__ import annotations

from lle.tiles import Direction

from ._candidates import CandidateLayout
from ._constructive import _ConstructiveGenerator


class _Level6StyleGenerator(_ConstructiveGenerator):
    _FLUSH_PROB = 0.75

    _WALL_SHAPES = (
        # (weight, offsets-from-anchor)
        (4, ((0, 0), (0, 1))),  # bar-2 horizontal
        (4, ((0, 0), (1, 0))),  # bar-2 vertical
        (1, ((0, 0), (0, 1), (0, 2))),  # bar-3 horizontal
        (1, ((0, 0), (1, 0), (2, 0))),  # bar-3 vertical
        (1, ((0, 0), (0, 1), (1, 0))),  # L
        (1, ((0, 0), (0, 1), (1, 1))),
        (1, ((0, 0), (1, 0), (1, 1))),
        (1, ((0, 1), (1, 0), (1, 1))),
        (2, ((0, 0), (0, 1), (1, 0), (1, 1))),  # 2x2 block
    )

    def __init__(
        self,
        *,
        width: int,
        height: int,
        n_agents: int = 2,
        n_lasers: int = 3,
        n_walls: int | None = None,
        t_max: int | None = None,
    ):
        if n_lasers < 1:
            raise ValueError(f"kind='level6_style' requires lasers >= 1; got {n_lasers}.")
        super().__init__(
            width=width,
            height=height,
            n_agents=n_agents,
            n_lasers=n_lasers,
            cooperative=True,
            n_walls=n_walls,
            t_max=t_max,
        )

    def _flush_offset(self, max_offset: int) -> int:
        """0 (flush against the edge) most of the time, 1-cell margin sometimes."""
        margin = 0 if self._rng.random() < self._FLUSH_PROB else 1
        return min(margin, max(0, max_offset))

    def _cluster_shape(self) -> tuple[int, int]:
        """Return (rows, cols) of the cluster rectangle to fit ``self.agents`` cells."""
        n = self.agents
        if n >= 4:
            return (2, 2)
        if n == 3:
            return (1, 3)
        if n == 2:
            return (1, 2)
        return (1, 1)

    @staticmethod
    def _cluster_cells(top_left, shape):
        r, c = top_left
        h, w = shape
        return [(r + dr, c + dc) for dr in range(h) for dc in range(w)]

    def _make_cooperative_candidate_layout(self) -> CandidateLayout | None:
        if self.agents < 1 or self.n_lasers < 1:
            return None
        cluster_h, cluster_w = self._cluster_shape()

        # Need enough room: start cluster + corridor (>= self.lasers cells) + exit cluster.
        if self.rows < cluster_h * 2 + max(self.n_lasers, 2) or self.cols < cluster_w + 2:
            if self.cols < cluster_w * 2 + max(self.n_lasers, 2) or self.rows < cluster_h + 2:
                return None

        orientation = self._rng.choice(["vertical", "horizontal"])

        if orientation == "vertical":
            third = max(cluster_h, self.rows // 3)
            if third + cluster_h > self.rows:
                return None
            start_row_max = max(0, third - cluster_h)
            exit_row_min = self.rows - third
            exit_row_slack = max(0, self.rows - cluster_h - exit_row_min)
            start_row = self._flush_offset(start_row_max)
            exit_row = self.rows - cluster_h - self._flush_offset(exit_row_slack)
            start_col = self._rng.randint(0, self.cols - cluster_w)
            exit_col = self._rng.randint(0, self.cols - cluster_w)
            start_tl = (start_row, start_col)
            exit_tl = (exit_row, exit_col)
            corridor_axis = "row"
        else:
            third = max(cluster_w, self.cols // 3)
            if third + cluster_w > self.cols:
                return None
            start_col_max = max(0, third - cluster_w)
            exit_col_min = self.cols - third
            exit_col_slack = max(0, self.cols - cluster_w - exit_col_min)
            start_col = self._flush_offset(start_col_max)
            exit_col = self.cols - cluster_w - self._flush_offset(exit_col_slack)
            start_row = self._rng.randint(0, self.rows - cluster_h)
            exit_row = self._rng.randint(0, self.rows - cluster_h)
            start_tl = (start_row, start_col)
            exit_tl = (exit_row, exit_col)
            corridor_axis = "col"

        agent_cells = self._cluster_cells(start_tl, (cluster_h, cluster_w))[: self.agents]
        exit_cells = self._cluster_cells(exit_tl, (cluster_h, cluster_w))[: self.agents]

        # Sanity: clusters must not overlap.
        if set(agent_cells) & set(exit_cells):
            return None

        reserved: set[tuple[int, int]] = set(agent_cells) | set(exit_cells)
        lasers: list[tuple[int, tuple[int, int], Direction]] = []

        if corridor_axis == "row":
            start_bottom = max(r for r, _ in agent_cells)
            exit_top = min(r for r, _ in exit_cells)
            corridor = list(range(start_bottom + 1, exit_top))
            if len(corridor) < self.n_lasers:
                return None
            self._rng.shuffle(corridor)
            chosen = sorted(corridor[: self.n_lasers])
            for i, r in enumerate(chosen):
                if i % 2 == 0:
                    src = (r, 0)
                    direction = Direction.EAST
                else:
                    src = (r, self.cols - 1)
                    direction = Direction.WEST
                if src in reserved:
                    return None
                reserved.add(src)
                lasers.append((i, src, direction))
        else:
            start_right = max(c for _, c in agent_cells)
            exit_left = min(c for _, c in exit_cells)
            corridor = list(range(start_right + 1, exit_left))
            if len(corridor) < self.n_lasers:
                return None
            self._rng.shuffle(corridor)
            chosen = sorted(corridor[: self.n_lasers])
            for i, c in enumerate(chosen):
                if i % 2 == 0:
                    src = (0, c)
                    direction = Direction.SOUTH
                else:
                    src = (self.rows - 1, c)
                    direction = Direction.NORTH
                if src in reserved:
                    return None
                reserved.add(src)
                lasers.append((i, src, direction))

        # Place walls as connected mini-shapes (bars / L / 2x2), within budget.
        free_cells = [(r, c) for r in range(self.rows) for c in range(self.cols) if (r, c) not in reserved]
        walls = self._place_wall_shapes(free_cells, self.n_walls)

        return CandidateLayout(
            agents=agent_cells,
            exits=exit_cells,
            walls=walls,
            lasers=lasers,
        )

    def _place_wall_shapes(
        self,
        free_cells: list[tuple[int, int]],
        budget: int,
    ) -> list[tuple[int, int]]:
        """Place walls as connected mini-shapes (bars / L / 2x2), within budget."""
        free_set = set(free_cells)
        anchors = list(free_cells)
        self._rng.shuffle(anchors)
        weights = [w for w, _ in self._WALL_SHAPES]
        shapes = [s for _, s in self._WALL_SHAPES]
        walls: list[tuple[int, int]] = []

        for anchor in anchors:
            if budget <= 0:
                break
            if anchor not in free_set:
                continue
            chosen_cells: list[tuple[int, int]] | None = None
            for shape in self._rng.choices(shapes, weights=weights, k=4):
                if len(shape) > budget:
                    continue
                cells = [(anchor[0] + dr, anchor[1] + dc) for dr, dc in shape]
                if all(cell in free_set for cell in cells):
                    chosen_cells = cells
                    break
            if chosen_cells is None:
                chosen_cells = [anchor]
            for cell in chosen_cells:
                free_set.discard(cell)
            walls.extend(chosen_cells)
            budget -= len(chosen_cells)

        return walls
