from .constraint import ConstraintGenerator


class InitializationConstraints(ConstraintGenerator):
    """Initial SAT constraints for agent and laser state."""

    def generate(self, t: int) -> list[list[int]]:
        if t == 0:
            # Each agent is at its starting position at time 0, i.e. one clause per agent
            return [[self.var.agent(agent, x, y, 0)] for agent, (x, y) in enumerate(self.ctx.world.start_pos)]
        return []
