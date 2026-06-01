from .base import Constraint


class InitializationConstraints(Constraint):
    """Initial SAT constraints for agent and laser state."""

    def generate(self):
        return [*self._agents_initial_position(), *self._lasers_initial_beam()]

    def _agents_initial_position(self):
        agent_var = self.ctx.agent_var
        for agent, (x, y) in self.ctx.agents:
            start = agent_var.get((agent.color, x, y, 0))
            if start is None:
                # The formula is not satisfiable, stop here.
                yield []
                return
            else:
                yield [start]

    def _lasers_initial_beam(self):
        beam_var = self.ctx.beam_var
        for laser, source in self.ctx.lasers:
            c = laser.color
            d = laser.direction
            x, y = source
            for t in range(self.t_max + 1):
                yield [beam_var[c, d, source, x, y, t]]
