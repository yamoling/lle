import itertools
from collections import deque

from lle.tiles import LaserSource
from lle.types import Position
from lle.world import World


def get_neighbours(world: World, pos: Position) -> list[Position]:
    """4-directional neighbors that are within bounds."""
    if pos in world.exit_pos:
        # When an agent reaches an exit, it can no longer move, so there is no other neighbour than itself.
        return [pos]
    i, j = pos
    result = []
    for di, dj in ((-1, 0), (1, 0), (0, -1), (0, 1)):
        ni, nj = i + di, j + dj
        if 0 <= ni < world.height and 0 <= nj < world.width and (ni, nj) not in world.wall_pos:
            result.append((ni, nj))
    return result


class ConstraintContext:
    """Pre-computed data shared across all constraint classes. Built once."""

    def __init__(self, world: World, t_max: int):
        self.world = world
        self.t_max = t_max

        all_positions = frozenset(itertools.product(range(world.height), range(world.width)))
        self.walls = frozenset(world.wall_pos)
        # self.agents = [(a, a.position) for a in _agents]
        self.exits = list[Position](world.exit_pos)
        self.valid_positions = all_positions.difference(self.walls).difference(world.void_pos)
        self.neighbours = {
            pos: [pos, *(n for n in get_neighbours(world, pos) if n in self.valid_positions)] for pos in self.valid_positions
        }
        # Reverse adjacency: predecessors[p] are the positions from which an agent can move into p
        # in a single step. This differs from `neighbours` for exit tiles, which an agent cannot
        # leave (so `neighbours[exit] == [exit]`) but can still be entered from adjacent cells.
        self.predecessors = {pos: set[Position]() for pos in self.valid_positions}
        for pos, successors in self.neighbours.items():
            for successor in successors:
                self.predecessors[successor].add(pos)
        # Time-wise reachability map
        self._reachable_positions = self.compute_time_reachability_map(world, t_max, self.neighbours)
        # The distance from each valid position to the nearest exit
        self._exit_distance = self.compute_exit_distance(world, self.predecessors)
        # A index i, the set of positions from which, at time step `i`, an exit can still be reached.
        self._exit_reachable = [
            {pos for pos, dist in self._exit_distance.items() if dist <= remaining} for remaining in range(t_max, -1, -1)
        ]
        # A cheap global lower bound on the shortest solution length: the maximum
        # actual walkable shortest-path distance from any agent to its nearest exit.
        self.solution_lower_bound = self.compute_solution_lower_bound(world.start_pos, self._exit_distance)
        self.laser_paths = {s: self._laser_path(world, s) for s in world.laser_sources}
        self.reachable_laser_paths = self.compute_reachable_lasers()
        self.prev_laser_beam = self.compute_effective_prev_laser()

    def get_prev_beam(self, t: int, i: int, j: int, laser_id: int):
        return self.prev_laser_beam.get((t, i, j, laser_id))

    @staticmethod
    def _laser_path(world: World, source: LaserSource) -> list[tuple[int, int]]:
        """Return the source tile followed by all beam tiles for one laser source."""
        path = []
        di, dj = source.direction.delta
        i = source.pos[0] + di
        j = source.pos[1] + dj
        while 0 <= i < world.height and 0 <= j < world.width and (i, j) not in world.wall_pos:
            path.append((i, j))
            i, j = i + di, j + dj
        return path

    def compute_reachable_lasers(self):
        reachable = dict[LaserSource, list[list[Position]]]()
        for source, path in self.laser_paths.items():
            time_wise = list[list[Position]]()
            for t in range(self.t_max + 1):
                reachable_by_blocking_agent = self.reachable_positions(t, source.agent_id)
                time_wise.append([p for p in path if p in reachable_by_blocking_agent])
            reachable[source] = time_wise
        return reachable

    def compute_effective_prev_laser(self):
        """
        The effective previous laser tile is the previous laser tile that is blockable.
        """
        prev_tiles = dict[tuple[int, int, int, int], tuple[int, int]]()
        for source, time_wise_reachable in self.reachable_laser_paths.items():
            for t, reachable_path in enumerate(time_wise_reachable):
                if len(reachable_path) == 0:
                    continue
                prev = reachable_path[0]
                for i, j in reachable_path[1:]:
                    prev_tiles[t, i, j, source.laser_id] = prev
                    prev = i, j
        return prev_tiles

    def reachable_positions(self, t: int, *agents: int) -> set[Position]:
        """Return positions that are reachable by the given agents exactly at time `t`, filtered by exit reachability."""
        if t < 0 or t > self.t_max or len(agents) == 0:
            return set()
        # Positions that the agent can reach and that can still yield an exit within the remaining time
        reachable = self._reachable_positions[agents[0]][t] & self._exit_reachable[t]
        for agent_num in agents[1:]:
            reachable = reachable & self._reachable_positions[agent_num][t]
        return reachable

    @staticmethod
    def compute_exit_distance(world: World, predecessors: dict[Position, set[Position]]):
        """
        Compute the minimum number of steps from each valid position to the nearest exit.

        This is a multi-source BFS seeded with all exit tiles at distance 0 and expanded
        over the *reverse* adjacency (`predecessors`): if an agent can move from ``p`` into
        ``cur`` in one step, then ``p`` is one step further from an exit than ``cur``. Because
        each cell is assigned a distance exactly once (when first dequeued), seeded exits keep
        their distance 0 even when two exits are adjacent to each other.
        """
        dist = {pos: 0.0 for pos in world.exit_pos}
        frontier = deque[Position](world.exit_pos)
        while len(frontier) > 0:
            current = frontier.popleft()
            for pred in predecessors.get(current, ()):
                if pred not in dist:
                    dist[pred] = dist[current] + 1
                    frontier.append(pred)
        return dist

    @staticmethod
    def compute_solution_lower_bound(start_pos: list[Position], exit_distances: dict[Position, float]):
        """Return a cheap admissible lower bound on the shortest plan length.

        The bound is the maximum over agents of the shortest walkable path
        distance from that agent's start position to any exit.
        """
        bound = 0.0
        for position in start_pos:
            distance = exit_distances.get(position, 0)
            if distance > bound:
                bound = distance
        return int(bound)

    @staticmethod
    def compute_time_reachability_map(world: World, t_max: int, neighbours: dict[Position, list[Position]]):
        """
        Compute agent-wise reachability over time using a forward flood fill.

        For each agent and each time step up to `t_max`, this builds set of reachable position
        starting from the agent's initial position and expanding one neighbour at a time.
        """
        reachable_positions = dict[int, list[set[Position]]]()
        for agent, start_pos in enumerate(world.start_pos):
            reachable: list[set[Position]] = [{start_pos}]
            for _t in range(t_max):
                frontier = reachable[-1]
                nxt = set()
                for pos in frontier:
                    nxt.update(neighbours[pos])
                reachable.append(nxt)
            reachable_positions[agent] = reachable
        return reachable_positions

    def can_stay(self, t: int, pos: Position):
        """Check if staying in the same position for one more timestep is still compatible with reaching an exit."""
        if t + 1 > self.t_max:
            return False
        return pos in self._exit_reachable[t + 1]

    def prev_neighbours(self, agent_num: int, i: int, j: int, t):
        """Return the positions the agent could have occupied at time step (t - 1) to reach (i, j) at t."""
        return self.reachable_positions(t - 1, agent_num).intersection(self.predecessors[i, j])
