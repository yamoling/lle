from .base import Constraint


class LaserConstraints(Constraint):
    def generate(self):
        return [
            *self._no_step_on_active_laser(),
            *self._beam_propagation(),
            *self._link_beam_and_laser(),
        ]

    def _no_step_on_active_laser(self):
        agent_var = self.ctx.agent_var
        laser_var = self.ctx.laser_var
        for laser, _ in self.ctx.lasers:
            for agent, _ in self.ctx.agents:
                c1, c2 = agent.color, laser.color
                if c1 == c2:
                    continue
                for t in range(self.t_max + 1):
                    for x, y in self.reachable_positions(t, c1):
                        yield [-agent_var[c1, x, y, t], -laser_var[c2, x, y, t]]

    def _beam_propagation(self):
        agent_var = self.ctx.agent_var
        beam_var = self.ctx.beam_var
        propagation_map = self.ctx.beam_propagation_map

        for laser, _ in self.ctx.lasers:
            c = laser.color
            d = laser.direction
            entries = propagation_map[c, d]
            for x, y, nx, ny, is_wall in entries:
                for t in range(self.t_max + 1):
                    if is_wall:
                        yield [-beam_var[c, d, nx, ny, t]]
                    else:
                        bv_src = beam_var[c, d, x, y, t]
                        bv_dst = beam_var[c, d, nx, ny, t]
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

    def _link_beam_and_laser(self):
        beam_var = self.ctx.beam_var
        laser_var = self.ctx.laser_var
        all_positions = self.ctx.all_positions

        for laser, _ in self.ctx.lasers:
            c = laser.color
            d = laser.direction
            for x, y in all_positions:
                for t in range(self.t_max + 1):
                    bv = beam_var[c, d, x, y, t]
                    lv = laser_var[c, x, y, t]
                    yield [-bv, lv]
                    yield [bv, -lv]


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
        propagation_map = self.ctx.beam_propagation_map

        for laser, _ in self.ctx.lasers:
            c = laser.color
            d = laser.direction
            entries = propagation_map[c, d]

            for x, y, nx, ny, is_wall in entries:
                for t in range(self.t_max + 1):
                    if is_wall:
                        yield [-beam_var[c, d, nx, ny, t]]
                    else:
                        bv_src = beam_var[c, d, x, y, t]
                        bv_dst = beam_var[c, d, nx, ny, t]
                        yield [-bv_src, bv_dst]
                        yield [bv_src, -bv_dst]
