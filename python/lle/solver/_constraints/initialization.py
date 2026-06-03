from lle.solver.variable_factory import VariableFactory

from .base import ConstraintContext, ConstraintGenerator


class InitializationConstraints(ConstraintGenerator):
    """Initial SAT constraints for agent and laser state."""

    def __init__(self, var: VariableFactory, ctx: ConstraintContext):
        super().__init__(var, ctx)

    def generate(self, t: int):
        clauses = []
        if t == 0:
            # Each agent is at its starting position at time 0, i.e. one clause per agent
            clauses.extend([[self.var.agent(agent, x, y, 0)] for agent, (x, y) in enumerate(self.ctx.world.start_pos)])

        # Laser source tiles are always active. Propagation constraints then determine
        # which downstream beam tiles are active or blocked at each time step.
        clauses.extend([[self.var.laser(source.laser_id, *source.pos, t)] for source in self.ctx.world.laser_sources])
        return clauses

    # def _lasers_initial_beam(self, t_min: int, t_max: int):
    #     beam_var = self.ctx.beam_var
    #     for laser, source in self.ctx.lasers:
    #         c = laser.color
    #         d = laser.direction
    #         x, y = source
    #         for t in range(max(0, t_min), min(self.t_max, t_max) + 1):
    #             yield [beam_var[c, d, source, x, y, t]]
