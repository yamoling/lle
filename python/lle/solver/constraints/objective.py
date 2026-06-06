from ..variable_factory import VariableFactory
from .constraint import ConstraintGenerator
from .context import ConstraintContext


class ObjectiveGenerator(ConstraintGenerator):
    def __init__(self, var: VariableFactory, ctx: ConstraintContext):
        super().__init__(var, ctx)
        self.n_agents = ctx.world.n_agents
        self.exits = set(ctx.world.exit_pos)

    def generate(self, t: int):
        """
        At the final time step, all agents must be on an exit.
        This ensures that the solver finds solutions where agents actually reach exits.
        """
        res = []
        for agent in range(self.n_agents):
            possible_exits = self.exits.intersection(self.reachable_positions(t, agent))
            res.append([self.var.agent(agent, x, y, t) for x, y in possible_exits])
        return res
