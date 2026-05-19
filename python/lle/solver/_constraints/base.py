from abc import ABC, abstractmethod
from collections import deque

from lle.world import World

from .._internal import (
    Position,
    VariableFactory,
    agents_from_world,
    all_positions,
    get_neighbors,
    is_within_bounds,
    laser_sources_from_world,
)


class ConstraintContext:
    """Pre-computed data shared across all constraint classes. Built once."""

    def __init__(self, world: World, var_factory: VariableFactory, t_max: int):
        self.world = world
        self.var = var_factory
        self.t_max = t_max

        # Pre-compute sets
        self.walls = frozenset(world.wall_pos)
        _agents = agents_from_world(world)
        _lasers = laser_sources_from_world(world)
        self.laser_positions = frozenset(src.position for src in _lasers)
        self.blocked = self.walls | self.laser_positions
        self.agents = [(a, a.position) for a in _agents]
        self.lasers = [(src, src.position) for src in _lasers]
        self.exits = list(world.exit_pos)
        self.all_positions = all_positions(world)
        self.valid_positions = [p for p in self.all_positions if p not in self.blocked]

        # Pre-compute neighbor map: pos -> [pos] + unblocked neighbors
        self.neighbor_map = dict[Position, list[Position]]()
        for pos in self.valid_positions:
            neighbors = [n for n in get_neighbors(world, pos) if n not in self.blocked]
            self.neighbor_map[pos] = [pos] + neighbors

        # Pre-compute time-reachable positions for each agent.
        # We only generate variables for locations that can actually be reached
        # from the initial position by following legal moves on the grid and
        # still reach an exit within the remaining horizon.
        self._reachable_positions: dict[int, list[set[Position]]] = {}
        for agent, _ in self.agents:
            c = agent.color
            reachable: list[set[Position]] = [{agent.position}]
            for _t in range(t_max):
                frontier = reachable[-1]
                nxt: set[Position] = set()
                for pos in frontier:
                    nxt.update(self.neighbor_map[pos])
                reachable.append(nxt)
            self._reachable_positions[c] = reachable

        self._exit_distance: dict[Position, int] = {pos: t_max + 1 for pos in self.valid_positions}
        queue = deque()
        for exit_pos in self.exits:
            if exit_pos in self._exit_distance:
                self._exit_distance[exit_pos] = 0
                queue.append(exit_pos)
        while queue:
            pos = queue.popleft()
            dist = self._exit_distance[pos]
            for nxt in get_neighbors(world, pos):
                if nxt in self.blocked or nxt not in self._exit_distance:
                    continue
                if self._exit_distance[nxt] > dist + 1:
                    self._exit_distance[nxt] = dist + 1
                    queue.append(nxt)

        self._exit_reachable: list[set[Position]] = [set() for _ in range(t_max + 1)]
        for pos, dist in self._exit_distance.items():
            if dist > t_max:
                continue
            for remaining in range(dist, t_max + 1):
                self._exit_reachable[remaining].add(pos)

        # Pre-compute variable IDs
        self.agent_var = dict[tuple[int, int, int, int], int]()
        for agent, _ in self.agents:
            c = agent.color
            for t in range(t_max + 1):
                for x, y in self.reachable_positions(t, agent.color):
                    self.agent_var[c, x, y, t] = var_factory.agent(c, x, y, t)

        # Pre-compute beam rays per laser.
        # Each laser has a single straight path until the first wall, boundary,
        # or laser source tile. Beam and laser occupancy variables are only
        # generated on that path.
        self.beam_paths: dict[tuple[int, tuple[int, int]], list[Position]] = {}
        for laser, _ in self.lasers:
            key = (laser.color, laser.direction)
            di, dj = laser.direction
            x, y = laser.position
            path: list[Position] = [(x, y)]
            while True:
                nx = x + di
                ny = y + dj
                if not is_within_bounds(world, (nx, ny)):
                    break
                if (nx, ny) in self.walls or (nx, ny) in self.laser_positions:
                    break
                path.append((nx, ny))
                x, y = nx, ny
            self.beam_paths[key] = path

        # `laser_var` used to mirror beam occupancy, but the encoding now uses
        # the beam occupancy directly. Keep the attribute for compatibility.
        self.laser_var = {}

        self.beam_var = dict[tuple[int, tuple[int, int], int, int, int], int]()
        for laser, _ in self.lasers:
            c = laser.color
            d = laser.direction
            for t in range(t_max + 1):
                for x, y in self.beam_paths[c, d]:
                    self.beam_var[c, d, x, y, t] = var_factory.beam(c, d, x, y, t)

    def reachable_positions(self, t: int, *agents: int) -> set[Position]:
        """Return positions that are reachable by `agent_num` exactly at time `t`."""
        if t < 0 or t > self.t_max:
            return set()
        reachable = self._reachable_positions[agents[0]][t]
        for agent_num in agents[1:]:
            reachable = reachable.intersection(self._reachable_positions[agent_num][t])
        return reachable


class Constraint(ABC):
    def __init__(self, ctx: ConstraintContext):
        self.ctx = ctx
        self.world = ctx.world
        self.var = ctx.var
        self.t_max = ctx.t_max

    @abstractmethod
    def generate(self):
        return []

    def _profile_method(self, _method_name: str, method_func):
        return list(method_func())

    def reachable_positions(self, t: int, *agents: int):
        return self.ctx.reachable_positions(t, *agents)
