from .base import Constraint


class InitializationConstraints(Constraint):
    def generate(self):
        return [*self._agents_initial_position(), *self._lasers_initial_beam()]

    def _agents_initial_position(self):
        agent_var = self.ctx.agent_var
        all_positions = self.ctx.all_positions
        for agent, (x, y) in self.ctx.agents:
            c = agent.color
            yield [agent_var[c, x, y, 0]]
            for ox, oy in all_positions:
                if (ox, oy) != (x, y):
                    yield [-agent_var[c, ox, oy, 0]]

    def _lasers_initial_beam(self):
        beam_var = self.ctx.beam_var
        for laser, (x, y) in self.ctx.lasers:
            c = laser.color
            d = laser.direction
            for t in range(self.t_max + 1):
                yield [beam_var[c, d, x, y, t]]
