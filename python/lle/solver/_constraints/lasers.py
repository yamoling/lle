from collections.abc import Iterator

from lle.solver.variable_factory import VariableFactory
from lle.tiles import LaserSource

from .base import ConstraintContext, ConstraintGenerator


class LaserConstraints(ConstraintGenerator):
    """SAT constraints that model laser propagation and blocking."""

    def __init__(self, var: VariableFactory, ctx: ConstraintContext):
        super().__init__(var, ctx)
        self.sources: list[LaserSource] = ctx.world.laser_sources
        self.n_agents = ctx.world.n_agents

    def generate(self, t: int):
        return [
            *self._no_step_on_active_laser(t),
            *self._beam_propagation(t),
        ]

    def _laser_path(self, source: LaserSource) -> list[tuple[int, int]]:
        """Return the source tile followed by all beam tiles for one laser source."""
        path = [source.pos]
        dx, dy = source.direction.delta
        x = source.pos[0] + dx
        y = source.pos[1] + dy
        while 0 <= x < self.world.height and 0 <= y < self.world.width and (x, y) not in self.ctx.walls:
            path.append((x, y))
            x, y = x + dx, y + dy
        return path

    def _laser_edges(self, source: LaserSource) -> Iterator[tuple[tuple[int, int], tuple[int, int]]]:
        path = self._laser_path(source)
        yield from zip(path, path[1:])

    def _no_step_on_active_laser(self, t: int):
        r"""
        Agents cannot step on an active laser beam of another colour.

        Formula: agent(a, x, y, t) -> ¬laser(l, x, y, t)
        """
        for source in self.sources:
            path = self._laser_path(source)
            for agent in range(self.n_agents):
                if agent == source.agent_id:
                    continue
                for x, y in path:
                    if (x, y) not in self.reachable_positions_for_agent(t, agent):
                        continue
                    yield [-self.var.agent(agent, x, y, t), -self.var.laser(source.laser_id, x, y, t)]

    def _beam_propagation(self, t: int):
        r"""
        Laser beams propagate from the source until blocked by a same-colour agent.

        For each edge src -> dst in a laser ray:
        - if no same-colour agent can occupy dst, src and dst have the same laser state;
        - if a same-colour agent can occupy dst, it may block dst, otherwise dst follows src.
        """
        for source in self.sources:
            for (x, y), (nx, ny) in self._laser_edges(source):
                src_var = self.var.laser(source.laser_id, x, y, t)
                dst_var = self.var.laser(source.laser_id, nx, ny, t)
                agent_can_block = (nx, ny) in self.reachable_positions_for_agent(t, source.agent_id)
                if agent_can_block:
                    agent_var = self.var.agent(source.agent_id, nx, ny, t)
                    yield [-src_var, agent_var, dst_var]
                    yield [-agent_var, -dst_var]
                else:
                    yield [-src_var, dst_var]
                yield [src_var, -dst_var]


class StrictLaserConstraints(LaserConstraints):
    """
    Variant of LaserConstraints where beam propagation does NOT stop on agents.
    It only stops at walls / bounds (same as base behavior except agent blocking).
    """

    def generate(self, t: int):
        return [
            *self._no_step_on_active_laser(t),
            *self._beam_propagation(t),
            *self.forbid_agent_walk_on_tile_with_laser_of_other_colour(t),
        ]

    def _beam_propagation(self, t: int):
        for source in self.sources:
            for (x, y), (nx, ny) in self._laser_edges(source):
                src_var = self.var.laser(source.laser_id, x, y, t)
                dst_var = self.var.laser(source.laser_id, nx, ny, t)
                yield [-src_var, dst_var]
                yield [src_var, -dst_var]

    def forbid_agent_walk_on_tile_with_laser_of_other_colour(self, t: int):
        for source in self.sources:
            for agent in range(self.n_agents):
                if agent == source.agent_id:
                    continue
                for x, y in self._laser_path(source):
                    if (x, y) not in self.reachable_positions_for_agent(t, agent):
                        continue
                    yield [-self.var.agent(agent, x, y, t)]
