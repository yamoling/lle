from abc import ABC, abstractmethod
from collections import deque
from typing import Collection

from lle.world import World

from .._internal import (
    AgentData,
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
        # Neighbour map
        self.neighbours = {pos: [pos, *(n for n in get_neighbors(world, pos) if n not in self.blocked)] for pos in self.valid_positions}
        # Time-wise reachability map
        self._reachable_positions = self.compute_time_reachability_map([a for a, _ in self.agents], t_max, self.neighbours)
        # The distance from each valid position to the nearest exit
        self._exit_distance = self.compute_exit_distance(world.exit_pos, self.valid_positions, t_max, self.neighbours)
        # A index i, the set of positions from which, at time step `i`, an exit can still be reached.
        self._exit_reachable = [{pos for pos, dist in self._exit_distance.items() if dist <= remaining} for remaining in range(t_max + 1)]
        # Positions where staying for one extra timestep is still compatible with reaching an exit.
        self._stay_allowed = [{pos for pos, dist in self._exit_distance.items() if dist < (t_max - t)} for t in range(t_max)]
        # Pre-compute variable IDs
        self.agent_var, self.beam_paths, self.beam_var = self.initialize_variables()

    def reachable_positions_for_agent(self, t: int, agent_num: int) -> set[Position]:
        """"""
        if t < 0 or t > self.t_max:
            return set()
        return self._reachable_positions[agent_num][t].intersection(self._exit_reachable[self.t_max - t])

    def reachable_positions(self, t: int, *agents: int) -> set[Position]:
        """Return positions that are reachable by the given agents exactly at time `t`."""
        if t < 0 or t > self.t_max or not agents:
            return set()
        reachable = self.reachable_positions_for_agent(t, agents[0])
        for agent_num in agents[1:]:
            reachable = reachable.intersection(self.reachable_positions_for_agent(t, agent_num))
        return reachable

    def initialize_variables(self):
        agent_var = dict[tuple[int, int, int, int], int]()
        beam_paths = dict[tuple[int, tuple[int, int]], list[Position]]()
        beam_var = dict[tuple[int, tuple[int, int], int, int, int], int]()

        for agent, _ in self.agents:
            for t in range(self.t_max + 1):
                for x, y in self.reachable_positions_for_agent(t, agent.color):
                    agent_var[agent.color, x, y, t] = self.var.agent(agent.color, x, y, t)

        # Pre-compute beam rays per laser.
        # Each laser has a single straight path until the first wall, boundary,
        # or laser source tile. Beam and laser occupancy variables are only
        # generated on that path.
        for laser, _ in self.lasers:
            key = (laser.color, laser.direction)
            di, dj = laser.direction
            x, y = laser.position
            path: list[Position] = [(x, y)]
            while True:
                nx = x + di
                ny = y + dj
                if not is_within_bounds(self.world, (nx, ny)):
                    break
                if (nx, ny) in self.walls or (nx, ny) in self.laser_positions:
                    break
                path.append((nx, ny))
                x, y = nx, ny
            beam_paths[key] = path

        for laser, _ in self.lasers:
            c = laser.color
            d = laser.direction
            for t in range(self.t_max + 1):
                for x, y in beam_paths[c, d]:
                    beam_var[c, d, x, y, t] = self.var.beam(c, d, x, y, t)
        return agent_var, beam_paths, beam_var

    @staticmethod
    def compute_exit_distance(exit_positions: list[Position], valid_positions: Collection[Position], t_max: int, neighbours: dict):
        """
        Compute the minimum number of steps from each valid position to the nearest exit using a breadth-first search.

        The search starts from all exit tiles at and expands through the precomputed neighbour map.
        """
        exit_distances = {pos: t_max + 1 for pos in valid_positions}
        queue = deque[Position]()
        for exit_pos in exit_positions:
            if exit_pos in exit_distances:
                exit_distances[exit_pos] = 0
                queue.append(exit_pos)
        while len(queue) > 0:
            pos = queue.popleft()
            dist = exit_distances[pos]
            for neighbour in neighbours[pos]:
                if neighbour not in exit_distances:
                    continue
                if exit_distances[neighbour] > dist + 1:
                    exit_distances[neighbour] = dist + 1
                    queue.append(neighbour)
        return exit_distances

    @staticmethod
    def compute_time_reachability_map(agents: list[AgentData], t_max: int, neighbours: dict[Position, list[Position]]):
        """
        Compute agent-wise reachability over time using a forward flood fill.

        For each agent and each time step up to `t_max`, this builds set of reachable position
        starting from the agent's initial position and expanding one neighbour at a time.
        """
        reachable_positions = dict[int, list[set[Position]]]()
        for agent in agents:
            reachable: list[set[Position]] = [{agent.position}]
            for _t in range(t_max):
                frontier = reachable[-1]
                nxt: set[Position] = set()
                for pos in frontier:
                    nxt.update(neighbours[pos])
                reachable.append(nxt)
            reachable_positions[agent.color] = reachable
        return reachable_positions


class Constraint(ABC):
    def __init__(self, ctx: ConstraintContext):
        self.ctx = ctx
        self.world = ctx.world
        self.var = ctx.var
        self.t_max = ctx.t_max

    @abstractmethod
    def generate(self) -> list:
        """Generate the clauses"""

    def _profile_method(self, _method_name: str, method_func):
        return list(method_func())

    def reachable_positions_for_agent(self, t: int, agent_num: int):
        return self.ctx.reachable_positions_for_agent(t, agent_num)

    def reachable_positions(self, t: int, *agents: int):
        return self.ctx.reachable_positions(t, *agents)

    def can_stay(self, t: int, pos: Position):
        """Check if staying in the same position for one more timestep is still compatible with reaching an exit."""
        return pos in self.ctx._stay_allowed[t]
