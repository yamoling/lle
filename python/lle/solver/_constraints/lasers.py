from .base import Constraint


class LaserConstraints(Constraint):
    """SAT constraints that model laser propagation and blocking."""

    def generate(self):
        return [
            *self._no_step_on_active_laser(),
            *self._beam_propagation(),
        ]

    def _no_step_on_active_laser(self):
        agent_var = self.ctx.agent_var
        beam_var = self.ctx.beam_var
        for laser, source in self.ctx.lasers:
            c2 = laser.color
            d = laser.direction
            path = self.ctx.beam_paths[c2, d, source]
            for agent, _ in self.ctx.agents:
                c1 = agent.color
                if c1 == c2:
                    continue
                for t in range(self.t_max + 1):
                    for x, y in path:
                        if (c1, x, y, t) in agent_var:
                            yield [-agent_var[c1, x, y, t], -beam_var[c2, d, source, x, y, t]]

    def _beam_propagation(self):
        agent_var = self.ctx.agent_var
        beam_var = self.ctx.beam_var

        for laser, source in self.ctx.lasers:
            c = laser.color
            d = laser.direction
            path = self.ctx.beam_paths[c, d, source]
            for (x, y), (nx, ny) in zip(path, path[1:]):
                for t in range(self.t_max + 1):
                    bv_src = beam_var[c, d, source, x, y, t]
                    bv_dst = beam_var[c, d, source, nx, ny, t]
                    # The beam must propagate forward.
                    # If a same-color agent can occupy the destination cell, it may block the beam there.
                    av_dst = agent_var.get((c, nx, ny, t), None)
                    if av_dst is not None:
                        yield [-bv_src, av_dst, bv_dst]
                        yield [-av_dst, -bv_dst]
                    else:
                        yield [-bv_src, bv_dst]
                    # If the beam is present at the destination, then it must have been present at the source.
                    yield [bv_src, -bv_dst]


class StrictLaserConstraints(LaserConstraints):
    """
    Variant of LaserConstraints where beam propagation does NOT stop on agents.
    It only stops at walls / bounds (same as base behavior except agent blocking).
    """

    def _beam_propagation(self):
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
                for t in range(self.t_max + 1):
                    bv_src = beam_var[c, d, source, x, y, t]
                    bv_dst = beam_var[c, d, source, nx, ny, t]
                    yield [-bv_src, bv_dst]
                    yield [bv_src, -bv_dst]
