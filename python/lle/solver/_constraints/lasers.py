from lle.solver.variable_factory import VariableFactory
from lle.tiles import Laser

from .base import ConstraintContext, ConstraintGenerator


class LaserConstraints(ConstraintGenerator):
    """SAT constraints that model laser propagation and blocking."""

    def __init__(self, var: VariableFactory, ctx: ConstraintContext):
        super().__init__(var, ctx)
        self.lasers: list[Laser] = ctx.world.lasers
        self.n_agents = ctx.world.n_agents

    def generate(self, t: int):
        return [
            *self._no_step_on_deadly_laser(t),
            *self._beam_propagation(t),
        ]

    def _no_step_on_deadly_laser(self, t: int):
        r"""
        The agent can not step on a laser of another colour,
        i.e. if the laser is active, then the agent cannot step on it.

        Formula explanation:
        -------------------
        The formula can also be read as "if the agent is in that
            - $\lnot agent(a, x, y, t) \rightarrow \lnot laser(l, x, y, t)$
        """
        for agent in range(self.n_agents):
            reachable_positions = self.reachable_positions(t, agent)
            for laser in self.lasers:
                # Ignore lasers that can be blocked by the agent
                if laser.agent_id == agent:
                    continue
                # Ignore lasers that are not reachable at time step t
                if laser.pos not in reachable_positions:
                    continue
                x, y = laser.pos
                agent_var = self.var.agent(agent, x, y, t)
                laser_var = self.var.laser(laser.laser_id, x, y, t)
                yield [-agent_var, -laser_var]

    def _beam_propagation(self, t: int):
        r"""
        1. If an agent walks into a laser of the same colour, it blocks it.
        2. When a tile is blocked, the next tile is also blocked.

        Formula:
        -------
        For every tile laser l where l.colour == agent:
            - $agent(colour, x, y, t) -> \lnot laser(l, x, y, t)$
            - $\lnot laser(l, x, y, t) \rightarrow \lnot laser(l, x + dx, y + dy, t + 1)$
        """
        for agent in range(self.n_agents):
            reachable_positions = self.reachable_positions(t, agent)
            for laser in self.lasers:
                # Only consider lasers that can be blocked
                if laser.agent_id != agent:
                    continue
                # Ignore lasers that are not reachable at time step t
                if laser.pos not in reachable_positions:
                    continue
                x, y = laser.pos
                # If an agent is on that tile, then it is disabled
                laser_var = self.var.laser(laser.laser_id, x, y, t)
                yield [-self.var.agent(agent, x, y, t), -laser_var]

                next_pos = self.ctx.get_next_laser_tile(x, y, laser.laser_id)
                if next_pos is None:
                    continue
                # If the tile is disabled, then the next tile is disabled as well (!a => !b) = a ∨ ¬b
                neighbour_var = self.var.laser(laser.laser_id, *next_pos, t)
                yield [laser_var, -neighbour_var]


class StrictLaserConstraints(LaserConstraints):
    """
    Variant of LaserConstraints where beam propagation does NOT stop on agents.
    It only stops at walls / bounds (same as base behavior except agent blocking).
    """

    def _beam_propagation(self, t: int):
        """
        Override only this method from LaserConstraints.
        Keep everything else exactly as in the parent class.
        """

        beam_var = self.ctx.beam_var

        for laser, source in self.ctx.lasers:
            c = laser.color
            d = laser.direction
            path = self.ctx.beam_paths[c, d, source]

            for (x, y), (nx, ny) in zip(path, path[1:]):
                for t in range(max(0, t_min), min(self.t_max, t_max) + 1):
                    bv_src = beam_var[c, d, source, x, y, t]
                    bv_dst = beam_var[c, d, source, nx, ny, t]
                    yield [-bv_src, bv_dst]
                    yield [bv_src, -bv_dst]
