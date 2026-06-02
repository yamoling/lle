from lle.solver.variable_factory import VariableFactory
from lle.tiles import Laser

from .base import ConstraintContext, ConstraintGenerator


class LaserConstraints(ConstraintGenerator):
    """SAT constraints that model laser propagation and blocking."""

    def __init__(self, var: VariableFactory, ctx: ConstraintContext):
        super().__init__(var, ctx)
        # self.lasers = ctx.lasers
        self.lasers: list[Laser] = ctx.world.lasers
        self.n_agents = ctx.world.n_agents

    def generate(self, t: int):
        return [
            *self._no_step_on_active_laser(t),
            *self._beam_propagation(t),
        ]

    def _no_step_on_active_laser(self, t: int):
        """The agent can not step on an active laser of another colour"""
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
                direction = laser.direction.delta
                laser_var = self.var.laser(laser.agent_id, direction, x, y, t)
                yield [-agent_var, -laser_var]

    def _get_laser_path(self, laser: Laser) -> list[tuple[int, int]]:
        """Generate the path of a laser beam until it hits a wall or the grid boundary."""
        path = []
        x, y = laser.pos
        dx, dy = laser.direction.delta
        width, height = self.ctx.world.grid.shape

        while 0 <= x < width and 0 <= y < height:
            path.append((x, y))
            # Check if the next position is a wall
            if self.ctx.world.grid[x, y].is_wall:
                break
            x += dx
            y += dy

        return path

    def _beam_propagation(self, t: int):
        """
        If an agent blocks a laser (i.e. on a laser tile of the same colour), then all subsequent tiles are disabled.
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

                # Get the laser's direction and color
                direction = laser.direction.delta
                colour = laser.agent_id

                # Generate the beam path starting from the laser's position
                path = self._get_laser_path(laser)

                # Iterate over the path to generate clauses
                for i in range(len(path)):
                    current_x, current_y = path[i]

                    # If this is not the last position in the path, propagate to the next position
                    if i < len(path) - 1:
                        next_x, next_y = path[i + 1]

                        # blocked[current] → ¬laser[current]
                        blocked_var = self.var.agent(agent, current_x, current_y, t)
                        laser_current_var = self.var.laser(colour, direction, current_x, current_y, t)
                        yield [-blocked_var, -laser_current_var]

                        # blocked[current] → ¬laser[next]
                        laser_next_var = self.var.laser(colour, direction, next_x, next_y, t)
                        yield [-blocked_var, -laser_next_var]

                        # ¬laser[current] → ¬laser[next] (recursive propagation)
                        yield [laser_current_var, -laser_next_var]

                        # ¬laser[next] → ¬laser[current] (bidirectional)
                        yield [laser_next_var, -laser_current_var]
                    else:
                        # Last position in the path: blocked[current] → ¬laser[current]
                        blocked_var = self.var.agent(agent, current_x, current_y, t)
                        laser_current_var = self.var.laser(colour, direction, current_x, current_y, t)
                        yield [-blocked_var, -laser_current_var]


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
