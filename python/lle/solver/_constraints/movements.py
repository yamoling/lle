from .base import Constraint, ConstraintContext

# Movement method constants
METHOD_LOCAL = "local"
METHOD_GLOBAL = "global"


class MovementConstraints(Constraint):
    def __init__(self, ctx: ConstraintContext, movement_method=METHOD_LOCAL):
        super().__init__(ctx)
        self.movement_method = movement_method

    def generate(self):
        all_clauses = []

        if self.movement_method == METHOD_LOCAL:
            all_clauses.extend(self._movement_rules_local())
        elif self.movement_method == METHOD_GLOBAL:
            all_clauses.extend(self._movement_rules_global())
            all_clauses.extend(self._unique_position())
        else:
            raise ValueError(f"Unknown movement method: {self.movement_method}")

        all_clauses.extend(self._no_overlap())
        all_clauses.extend(self._must_be_on_exit())
        all_clauses.extend(self._stays_on_exit())
        return all_clauses

    def _movement_rules_local(self):
        agent_var = self.ctx.agent_var
        neighbor_map = self.ctx.neighbor_map

        for agent, _ in self.ctx.agents:
            c = agent.color
            for t in range(self.t_max):
                t1 = t + 1
                # Forward: if at (x,y,t), must be at some neighbor at t+1.
                for x, y in self.reachable_positions(t, c):
                    n_pos = [(x, y) for x, y in neighbor_map[x, y] if (c, x, y, t1) in agent_var]
                    yield [-agent_var[c, x, y, t], *[agent_var[c, nx, ny, t1] for nx, ny in n_pos]]
                    # Inline uniqueness: pairwise exclusion on neighbors at t+1,
                    # i.e. there can only be one neighbor at t+1 if at (x,y,t)
                    n = len(n_pos)
                    for i in range(n):
                        x1, y1 = n_pos[i]
                        v1 = -agent_var[c, x1, y1, t1]
                        for j in range(i + 1, n):
                            x2, y2 = n_pos[j]
                            yield [v1, -agent_var[c, x2, y2, t1]]

                # Backward: if at (x,y,t+1), must have been at some neighbor at t.
                for x, y in self.reachable_positions(t1, c):
                    prev = [(x, y) for (x, y) in neighbor_map[x, y] if (c, x, y, t) in agent_var]
                    yield [-agent_var[c, x, y, t1]] + [agent_var[c, nx, ny, t] for nx, ny in prev]

    def _movement_rules_global(self):
        agent_var = self.ctx.agent_var
        neighbor_map = self.ctx.neighbor_map
        valid_positions = self.ctx.valid_positions

        for agent, _ in self.ctx.agents:
            c = agent.color
            for t in range(self.t_max):
                t1 = t + 1
                for x, y in valid_positions:
                    n_pos = neighbor_map[x, y]
                    yield [-agent_var[c, x, y, t]] + [agent_var[c, nx, ny, t1] for nx, ny in n_pos]

    def _unique_position(self):
        agent_var = self.ctx.agent_var
        all_positions = self.ctx.all_positions
        n = len(all_positions)

        for agent, _ in self.ctx.agents:
            c = agent.color
            for t in range(1, self.t_max + 1):
                for i in range(n):
                    x1, y1 = all_positions[i]
                    v1 = -agent_var[c, x1, y1, t]
                    for j in range(i + 1, n):
                        x2, y2 = all_positions[j]
                        yield [v1, -agent_var[c, x2, y2, t]]

    def _no_overlap(self):
        """
        Prevent agents from occupying the same cell at the same time,
        and from moving into a cell occupied by another agent in the
        previous timestep.
        """
        agent_var = self.ctx.agent_var
        agents = self.ctx.agents
        all_positions = self.ctx.all_positions
        n_agents = len(agents)

        for i in range(n_agents):
            c1 = agents[i][0].color
            for j in range(i + 1, n_agents):
                c2 = agents[j][0].color
                for t in range(self.t_max + 1):
                    t1 = t + 1
                    for x, y in all_positions:
                        v1_t = agent_var.get((c1, x, y, t))
                        v2_t = agent_var.get((c2, x, y, t))
                        if v1_t is not None and v2_t is not None:
                            yield [-v1_t, -v2_t]
                        v1_t1 = agent_var.get((c1, x, y, t1))
                        v2_t1 = agent_var.get((c2, x, y, t1))
                        if v1_t1 is not None and v2_t is not None:
                            yield [-v1_t1, -v2_t]
                        if v1_t is not None and v2_t1 is not None:
                            yield [-v1_t, -v2_t1]

    def _no_swapping_conflict(self):
        """Kept for compatibility; the core no-overlap rule already covers these cases."""
        return []

    def _must_be_on_exit(self):
        agent_var = self.ctx.agent_var
        for x, y in self.ctx.exits:
            yield [agent_var[agent.color, x, y, self.t_max] for agent, _ in self.ctx.agents if (agent.color, x, y, self.t_max) in agent_var]

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
