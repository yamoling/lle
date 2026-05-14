from abc import ABC, abstractmethod

from lle import World

from .._internal import (
    all_positions as _all_positions,
    get_neighbors as _get_neighbors,
    is_within_bounds as _is_within_bounds,
    agents_from_world,
    laser_sources_from_world,
)


class ConstraintContext:
    """Pre-computed data shared across all constraint classes. Built once."""

    def __init__(self, world: World, var_factory, T_MAX):
        self.world = world
        self.var = var_factory
        self.T_MAX = T_MAX

        # Pre-compute sets
        self.walls = frozenset(world.wall_pos)
        _agents = agents_from_world(world)
        _lasers = laser_sources_from_world(world)
        self.laser_positions = frozenset(src.position for src in _lasers)
        self.blocked = self.walls | self.laser_positions
        self.agents = [(a, a.position) for a in _agents]
        self.lasers = [(src, src.position) for src in _lasers]
        self.exits = list(world.exit_pos)
        self.all_positions = _all_positions(world)
        self.valid_positions = [p for p in self.all_positions if p not in self.blocked]

        # Pre-compute neighbor map: pos -> [pos] + unblocked neighbors
        self.neighbor_map = {}
        for pos in self.valid_positions:
            neighbors = [n for n in _get_neighbors(world, pos) if n not in self.blocked]
            self.neighbor_map[pos] = [pos] + neighbors

        # Pre-compute variable IDs
        self.agent_var = {}
        for agent, _ in self.agents:
            c = agent.color
            for t in range(T_MAX + 2):
                for x, y in self.all_positions:
                    self.agent_var[c, x, y, t] = var_factory.agent(c, x, y, t)

        self.laser_var = {}
        for laser, _ in self.lasers:
            c = laser.color
            for t in range(T_MAX + 1):
                for x, y in self.all_positions:
                    self.laser_var[c, x, y, t] = var_factory.laser(c, x, y, t)

        self.beam_var = {}
        for laser, _ in self.lasers:
            c = laser.color
            d = laser.direction
            for t in range(T_MAX + 1):
                for x, y in self.all_positions:
                    self.beam_var[c, d, x, y, t] = var_factory.beam(c, d, x, y, t)

        # Pre-compute beam propagation map per laser.
        # Beams never propagate into a laser source tile, which prevents a
        # boundary-facing source from generating a backward beam into the grid.
        self.beam_propagation_map = {}
        for laser, _ in self.lasers:
            key = (laser.color, laser.direction)
            entries = []
            di, dj = laser.direction  # already a (di, dj) tuple
            for x, y in self.all_positions:
                nx = x + di
                ny = y + dj
                if not _is_within_bounds(world, (nx, ny)):
                    continue
                if (nx, ny) in self.laser_positions:
                    continue
                is_blocker = (nx, ny) in self.walls
                entries.append((x, y, nx, ny, is_blocker))
            self.beam_propagation_map[key] = entries


class Constraint(ABC):
    def __init__(self, ctx: ConstraintContext):
        self.ctx = ctx
        self.world = ctx.world
        self.var = ctx.var
        self.T_MAX = ctx.T_MAX
        self.profiler = None

    def set_profiler(self, constraint_profiler):
        self.profiler = constraint_profiler

    @abstractmethod
    def generate(self):
        return []

    def _profile_method(self, method_name: str, method_func):
        if self.profiler:
            with self.profiler.profile_method(method_name) as method_profiler:
                clauses = method_profiler.count_clauses(method_func())
                return clauses
        else:
            return list(method_func())
