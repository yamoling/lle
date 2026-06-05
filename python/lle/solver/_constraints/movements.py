import itertools

from pysat.card import CardEnc

from lle.solver.variable_factory import VariableFactory

from .base import ConstraintContext, ConstraintGenerator
from .utils import implies

# Movement method constants
METHOD_LOCAL = "local"
METHOD_GLOBAL = "global"


class MovementConstraints(ConstraintGenerator):
    """SAT constraints for agent movement and collisions."""

    def __init__(self, var: VariableFactory, ctx: ConstraintContext):
        super().__init__(var, ctx)
        self.exits = set(ctx.exits)
        self.n_agents = ctx.world.n_agents

    def generate(self, t: int):
        return [
            *self._exactly_one_position(t),
            *self._time_wise_adjacency(t),
            *self._no_overlap(t),
            *self._no_following_conflict(t),
            *self._stays_on_exit(t),
        ]

    def _exactly_one_position(self, t: int):
        """Every agent is in exactly one position at any given time step."""
        for agent in range(self.n_agents):
            vars = [self.var.agent(agent, x, y, t) for (x, y) in self.reachable_positions(t, agent)]
            if len(vars) <= 1:
                continue
            # At least one position must be true
            yield list(vars)
            # At most one position is true
            yield from CardEnc.atmost(vars, bound=1, vpool=self.var.pool).clauses

    def _time_wise_adjacency(self, t: int):
        r"""
        If agent is at (x, y) at time step t, it must have been in an adjacent cell at t - 1.

        # Notes:
            - We only consider to remain in the same spot if at least one exit remains within reach at next step

        # Formula
        We want to express that A_t => (N_{1,t-1} V N_{2,t-1} V ... V N_{k,t-1}).
        For k=1 this degenerates to a simple implication A_t => N_{1,t-1}.
        """
        if t == 0:
            return
        for agent_num in range(self.n_agents):
            # previous_reachable = self.reachable_positions(t - 1, agent_num)
            for i, j in self.reachable_positions(t, agent_num):
                prev_pos = self.ctx.prev_neighbours(agent_num, i, j, t)
                current_var = self.var.agent(agent_num, i, j, t)
                if len(prev_pos) == 0:
                    # The agent cannot be in this location at this time step
                    raise ValueError("Does this ever happen ?")
                    yield [-current_var]
                    continue
                # if at (x,y,t), must have been at some reachable neighbour at t-1.
                # current_var -> prev1 or prev 2 or prev3 ...
                yield [-current_var, *(self.var.agent(agent_num, nx, ny, t - 1) for nx, ny in prev_pos)]

    def _no_overlap(self, t: int):
        """
        Prevent agents from occupying the same cell at the same time.

        # Formula
        Not in the same spot, i.e. ¬(agent1(x, y, t) ^ agent2(x, y, t)) <=> (¬agent1(x, y, t) ∨ ¬agent2(x, y, t))
        """
        for c1 in range(self.n_agents):
            for c2 in range(c1 + 1, self.n_agents):
                # Ensure that two agents can not be at the same position simultaneously
                for x, y in self.reachable_positions(t, c1, c2):
                    v1_t = self.var.agent(c1, x, y, t)
                    v2_t = self.var.agent(c2, x, y, t)
                    yield [-v1_t, -v2_t]

    def _no_following_conflict(self, t: int):
        """
        Ensure following conflicts are prevented, i.e. from moving into a cell occupied by another agent in the previous timestep.

        # Formula
        For all every combination of two agents (a1, a2), we must have:
            - a1(x, y, t) -> ¬ a2(x, y, t-1),
            - a2(x, y, t) -> ¬ a1(x, y, t-1),
        i.e. if an agent is at (x, y, t), then the other agent cannot have been at (x, y) at the
        previous time step.

        """
        if t == 0 or self.n_agents == 0:
            return
        for c1, c2 in itertools.combinations(range(self.n_agents), 2):
            for x, y in self.reachable_positions(t - 1, c1).intersection(self.reachable_positions(t, c2)):
                yield implies(self.var.agent(c2, x, y, t), -self.var.agent(c1, x, y, t - 1))
            for x, y in self.reachable_positions(t, c1).intersection(self.reachable_positions(t - 1, c2)):
                yield implies(self.var.agent(c1, x, y, t), -self.var.agent(c2, x, y, t - 1))

    def _stays_on_exit(self, t: int):
        """
        If agent was on an exit at t-1, it must also be on an exit at t.

        # Formula
        We have (agent on exit(t-1) -> agent on exit(t)), which can be written as:
            - ¬agent on exit(t-1) ∨ agent on exit(t)
        """
        if t == 0:
            return
        for agent in range(self.n_agents):
            reachable_exit_positions = self.reachable_positions(t - 1, agent).intersection(self.exits)
            for x, y in reachable_exit_positions:
                # Ensure that if the agent was on an exit at t-1, it remains on an exit at t
                yield [-self.var.agent(agent, x, y, t - 1), self.var.agent(agent, x, y, t)]
