from pysat.card import CardEnc

from .base import Constraint, ConstraintContext

# Movement method constants
METHOD_LOCAL = "local"
METHOD_GLOBAL = "global"


class MovementConstraints(Constraint):
    """SAT constraints for agent movement and collisions."""

    def __init__(self, ctx: ConstraintContext, movement_method=METHOD_LOCAL):
        super().__init__(ctx)
        self.movement_method = movement_method

    def generate(self):
        all_clauses = []

        # Exactly one position per agent and timestep.
        # (Replaces the previous backward/uniqueness clauses with a smaller cardinality encoding)
        all_clauses.extend(self._exactly_one_position())
        if self.movement_method == METHOD_LOCAL:
            all_clauses.extend(self._movement_rules_local())
        elif self.movement_method == METHOD_GLOBAL:
            all_clauses.extend(self._movement_rules_global())
        else:
            raise ValueError(f"Unknown movement method: {self.movement_method}")
        all_clauses.extend(self._no_overlap())
        all_clauses.extend(self._stays_on_exit())
        return all_clauses

    def _exactly_one_position(self):
        agent_var = self.ctx.agent_var
        for agent, _ in self.ctx.agents:
            for t in range(self.t_max + 1):
                lits = [agent_var[agent.color, x, y, t] for x, y in self.reachable_positions_for_agent(t, agent.color)]
                for clause in CardEnc.atmost(lits, bound=1, vpool=self.var.pool).clauses:
                    yield clause

    def _movement_rules_local(self):
        agent_var = self.ctx.agent_var
        neighbor_map = self.ctx.neighbours

        for agent, _ in self.ctx.agents:
            c = agent.color
            for t in range(self.t_max):
                t1 = t + 1
                # Forward: if at (x,y,t), must be at some neighbor at t+1.
                for pos in self.reachable_positions_for_agent(t, c):
                    # Only consider to STAY if it's still possible to reach an exit from there at t+1
                    n_pos = [
                        new_pos
                        for new_pos in neighbor_map[pos]
                        if (c, new_pos[0], new_pos[1], t1) in agent_var and (new_pos != pos or self.can_stay(t, pos))
                    ]
                    if not n_pos:
                        yield []
                        continue
                    yield [-agent_var[c, pos[0], pos[1], t], *[agent_var[c, nx, ny, t1] for nx, ny in n_pos]]

    def _movement_rules_global(self):
        agent_var = self.ctx.agent_var
        neighbor_map = self.ctx.neighbours
        valid_positions = self.ctx.valid_positions

        for agent, _ in self.ctx.agents:
            c = agent.color
            for t in range(self.t_max):
                t1 = t + 1
                for x, y in valid_positions:
                    src = agent_var.get((c, x, y, t))
                    if src is None:
                        continue
                    n_pos = [
                        pos
                        for pos in neighbor_map[x, y]
                        if (c, pos[0], pos[1], t1) in agent_var and (pos != (x, y) or self.can_stay(t, (x, y)))
                    ]
                    if not n_pos:
                        yield []
                        continue
                    yield [-src] + [agent_var[c, nx, ny, t1] for nx, ny in n_pos]

    def _no_overlap(self):
        """
        Prevent agents from occupying the same cell at the same time,
        and from moving into a cell occupied by another agent in the
        previous timestep.
        """
        agent_var = self.ctx.agent_var
        agents = self.ctx.agents
        n_agents = len(agents)

        for i in range(n_agents):
            c1 = agents[i][0].color
            for j in range(i + 1, n_agents):
                c2 = agents[j][0].color

                # Same-time collisions only matter on cells reachable by both agents.
                for t in range(self.t_max + 1):
                    for x, y in self.reachable_positions(t, c1, c2):
                        v1_t = agent_var.get((c1, x, y, t))
                        v2_t = agent_var.get((c2, x, y, t))
                        if v1_t is not None and v2_t is not None:
                            yield [-v1_t, -v2_t]

                # Cross-time collisions only matter on cells reachable by the two agents
                # at the relevant timesteps.
                for t in range(self.t_max):
                    t1 = t + 1
                    for x, y in self.reachable_positions_for_agent(t1, c1).intersection(self.reachable_positions_for_agent(t, c2)):
                        v1_t1 = agent_var.get((c1, x, y, t1))
                        v2_t = agent_var.get((c2, x, y, t))
                        if v1_t1 is not None and v2_t is not None:
                            yield [-v1_t1, -v2_t]
                    for x, y in self.reachable_positions_for_agent(t, c1).intersection(self.reachable_positions_for_agent(t1, c2)):
                        v1_t = agent_var.get((c1, x, y, t))
                        v2_t1 = agent_var.get((c2, x, y, t1))
                        if v1_t is not None and v2_t1 is not None:
                            yield [-v1_t, -v2_t1]

    def _must_be_on_exit(self):
        return []

    def _stays_on_exit(self):
        agent_var = self.ctx.agent_var
        for agent, _ in self.ctx.agents:
            c = agent.color
            for t in range(self.t_max):
                t1 = t + 1
                for x, y in self.ctx.exits:
                    key_t = c, x, y, t
                    if key_t not in agent_var:
                        continue
                    key_t1 = c, x, y, t1
                    if key_t1 not in agent_var:
                        continue
                    yield [-agent_var[key_t], agent_var[key_t1]]
