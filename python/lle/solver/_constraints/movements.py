from pysat.card import CardEnc

from lle.solver.variable_factory import VariableFactory

from .base import ConstraintContext, ConstraintGenerator

# Movement method constants
METHOD_LOCAL = "local"
METHOD_GLOBAL = "global"


class MovementConstraints(ConstraintGenerator):
    """SAT constraints for agent movement and collisions."""

    def __init__(self, var: VariableFactory, ctx: ConstraintContext):
        super().__init__(var, ctx)
        self.agents = ctx.agents
        self.exits = ctx.exits

    def generate(self, t: int):
        return [
            *self._exactly_one_position(t),
            *self._movement_rules_local(t),
            *self._no_overlap(t),
            *self._stays_on_exit(t),
        ]

    def _exactly_one_position(self, t: int):
        for agent, _ in self.agents:
            # For all reachable positions of each agent, there can only be one at every single time step that is true
            vars = [self.var.agent(agent.color, x, y, t) for (x, y) in self.reachable_positions_for_agent(t, agent.color)]
            for clause in CardEnc.atmost(vars, bound=1, vpool=self.var.pool).clauses:
                yield clause

    def _movement_rules_local(self, t: int):
        # agent_var = self.ctx.agent_var
        neighbor_map = self.ctx.neighbours

        for agent, _ in self.ctx.agents:
            c = agent.color
            for pos in self.reachable_positions_for_agent(t, c):
                # Only consider to STAY if it is still possible to reach an exit from there at t+1
                n_pos = [new_pos for new_pos in neighbor_map[pos] if new_pos != pos or self.can_stay(t, pos)]
                if len(n_pos) == 0:
                    yield []
                    continue
                x, y = pos
                # Forward: if at (x,y,t), must be at some neighbor at t+1.
                yield [-self.var.agent(c, x, y, t), *[self.var.agent(c, nx, ny, t + 1) for nx, ny in n_pos]]

    def _no_overlap(self, t: int):
        """
        Prevent agents from occupying the same cell at the same time,
        and from moving into a cell occupied by another agent in the
        previous timestep.
        """
        # agent_var = self.ctx.agent_var
        agents = self.ctx.agents
        n_agents = len(agents)

        for i in range(n_agents):
            c1 = agents[i][0].color
            for j in range(i + 1, n_agents):
                c2 = agents[j][0].color
                # Ensure that two agents can not be at the same position simultaneously
                for x, y in self.reachable_positions(t, c1, c2):
                    v1_t = self.var.agent(c1, x, y, t)
                    v2_t = self.var.agent(c2, x, y, t)
                    yield [-v1_t, -v2_t]

                # Ensure following conflicts are prevented
                t1 = t + 1
                for x, y in self.reachable_positions_for_agent(t1, c1).intersection(self.reachable_positions_for_agent(t, c2)):
                    v1_t1 = self.var.agent(c1, x, y, t1)
                    v2_t = self.var.agent(c2, x, y, t)
                    yield [-v1_t1, -v2_t]
                for x, y in self.reachable_positions_for_agent(t, c1).intersection(self.reachable_positions_for_agent(t1, c2)):
                    v1_t = self.var.agent(c1, x, y, t)
                    v2_t1 = self.var.agent(c2, x, y, t1)
                    yield [-v1_t, -v2_t1]

    def _stays_on_exit(self, t: int):
        for agent, _ in self.agents:
            c = agent.color
            positions = self.reachable_positions_for_agent(t, c)
            for x, y in self.exits:
                if (x, y) in positions:
                    # Ensure that the agent stays on the exit at time t and moves to the exit at time t+1
                    yield [-self.var.agent(c, x, y, t), self.var.agent(c, x, y, t + 1)]
