"""Base classes and shared context for SAT constraints.

Constraint classes use a shared context to avoid recomputing the same world
metadata, reachability sets, and SAT variable IDs.
"""

import itertools
from abc import ABC, abstractmethod
from collections import deque
from typing import Collection

from lle.world import World

from .._internal import (
    AgentData,
    Position,
    agents_from_world,
    get_neighbors,
)
from ..variable_factory import VariableFactory


class ConstraintContext:
    """Pre-computed data shared across all constraint classes. Built once."""

    def __init__(self, world: World, t_min: int, t_max: int):
        self.world = world
        self.t_min = t_min
        self.t_max = t_max

        _agents = agents_from_world(world)
        # _lasers = laser_sources_from_world(world)
        all_positions = frozenset(itertools.product(range(world.height), range(world.width)))
        self.blocked = frozenset(world.wall_pos)
        self.agents = [(a, a.position) for a in _agents]
        # self.lasers = [(src, src.position) for src in _lasers]
        self.exits = list[Position](world.exit_pos)
        self.valid_positions = all_positions.difference(self.blocked)
        # Neighbour map
        self.neighbours = {pos: [pos, *get_neighbors(world, pos)] for pos in self.valid_positions}
        # Time-wise reachability map
        self._reachable_positions = self.compute_time_reachability_map([a for a, _ in self.agents], t_max, self.neighbours)
        # The distance from each valid position to the nearest exit
        self._exit_distance = self.compute_exit_distance(world.exit_pos, self.valid_positions, t_max, self.neighbours)
        # A index i, the set of positions from which, at time step `i`, an exit can still be reached.
        self._exit_reachable = [{pos for pos, dist in self._exit_distance.items() if dist <= remaining} for remaining in range(t_max + 1)]
        # Positions where staying for one extra timestep is still compatible with reaching an exit.
        self._stay_allowed = [{pos for pos, dist in self._exit_distance.items() if dist < (t_max - t)} for t in range(t_max)]
        # A cheap global lower bound on the shortest solution length: the maximum
        # actual walkable shortest-path distance from any agent to its nearest exit.
        self.solution_lower_bound = self.compute_solution_lower_bound(self.agents, self._exit_distance)
        self.next_laser_tiles = self.compute_laser_paths(world)

    def get_next_laser_tile(self, x: int, y: int, laser_id: int):
        return self.next_laser_tiles.get((x, y, laser_id))

    @staticmethod
    def compute_laser_paths(world: World):
        next_tiles = dict[tuple[int, int, int], tuple[int, int]]()
        for source in world.laser_sources:
            dx, dy = source.direction.delta
            prev_x, prev_y = source.pos
            x = prev_x + dx
            y = prev_y + dy
            while (x, y) not in world.wall_pos and 0 <= x < world.height and 0 <= y < world.width:
                next_tiles[prev_x, prev_y, source.laser_id] = x, y
                prev_x, prev_y = x, y
                x, y = x + dx, y + dy
            print(next_tiles)
        return next_tiles

    def reachable_positions_for_agent(self, t: int, agent_num: int) -> set[Position]:
        if t < 0 or t > self.t_max:
            return set()
        # Return reachable positions without intersecting with _exit_reachable for testing
        return self._reachable_positions[agent_num][t]

    def reachable_positions(self, t: int, *agents: int) -> set[Position]:
        """Return positions that are reachable by the given agents exactly at time `t`."""
        if t < 0 or t > self.t_max or not agents:
            return set()
        reachable = self.reachable_positions_for_agent(t, agents[0])
        for agent_num in agents[1:]:
            reachable = reachable.intersection(self.reachable_positions_for_agent(t, agent_num))
        return reachable

    # def initialize_variables(self):
    #     agent_var = dict[tuple[int, int, int, int], int]()
    #     # Beam paths and variables are keyed by (colour, direction, source position).
    #     # Including the source position lets a colour have any number of laser sources,
    #     # even several sharing the same direction: each source keeps its own beam instead
    #     # of overwriting the others under a shared (colour, direction) key.
    #     beam_paths = dict[tuple[int, tuple[int, int], Position], list[Position]]()
    #     beam_var = dict[tuple[int, tuple[int, int], Position, int, int, int], int]()

    #     for agent, _ in self.agents:
    #         for t in range(self.t_max + 1):
    #             for x, y in self.reachable_positions_for_agent(t, agent.color):
    #                 agent_var[agent.color, x, y, t] = self.var.agent(agent.color, x, y, t)

    #     # Pre-compute beam rays per laser.
    #     # Each laser has a single straight path until the first wall, boundary,
    #     # or laser source tile. Beam and laser occupancy variables are only
    #     # generated on that path.
    #     for laser, source in self.lasers:
    #         key = (laser.color, laser.direction, source)
    #         di, dj = laser.direction
    #         x, y = laser.position
    #         path: list[Position] = [(x, y)]
    #         while True:
    #             nx = x + di
    #             ny = y + dj
    #             if not is_within_bounds(self.world, (nx, ny)):
    #                 break
    #             if (nx, ny) in self.walls or (nx, ny) in self.laser_positions:
    #                 break
    #             path.append((nx, ny))
    #             x, y = nx, ny
    #         beam_paths[key] = path

    #     for laser, source in self.lasers:
    #         c = laser.color
    #         d = laser.direction
    #         for t in range(self.t_max + 1):
    #             for x, y in beam_paths[c, d, source]:
    #                 beam_var[c, d, source, x, y, t] = self.var.beam(c, d, source, x, y, t)
    #     return agent_var, beam_paths, beam_var

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
    def compute_solution_lower_bound(
        agents: list[tuple[AgentData, Position]],
        exit_distances: dict[Position, int],
    ) -> int:
        """Return a cheap admissible lower bound on the shortest plan length.

        The bound is the maximum over agents of the shortest walkable path
        distance from that agent's start position to any exit.
        """
        if not agents or not exit_distances:
            return 0
        bound = 0
        for _, position in agents:
            distance = exit_distances.get(position, 0)
            if distance > bound:
                bound = distance
        return bound

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


class ConstraintGenerator(ABC):
    def __init__(self, var: VariableFactory, ctx: ConstraintContext):
        self.ctx = ctx
        self.world = ctx.world
        self.var = var

    @abstractmethod
    def generate(self, t: int) -> list:
        """Generate the clauses for the given time step."""

    def _profile_method(self, _method_name: str, method_func):
        return list(method_func())

    def reachable_positions_for_agent(self, t: int, agent_num: int):
        return self.ctx.reachable_positions_for_agent(t, agent_num)

    def reachable_positions(self, t: int, *agents: int):
        return self.ctx.reachable_positions(t, *agents)

    def can_stay(self, t: int, pos: Position):
        """Check if staying in the same position for one more timestep is still compatible with reaching an exit."""
        return pos in self.ctx._stay_allowed[t]
