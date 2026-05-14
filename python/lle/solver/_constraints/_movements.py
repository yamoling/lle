from ._base import Constraint, ConstraintContext

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
            all_clauses.extend(
                self._profile_method("movement_rules", self._movement_rules_local)
            )
        elif self.movement_method == METHOD_GLOBAL:
            all_clauses.extend(
                self._profile_method("movement_rules", self._movement_rules_global)
            )
            all_clauses.extend(
                self._profile_method("unique_position", self._unique_position)
            )
        else:
            raise ValueError(f"Unknown movement method: {self.movement_method}")

        all_clauses.extend(self._profile_method("no_overlap", self._no_overlap))
        all_clauses.extend(
            self._profile_method("must_be_on_exit", self._must_be_on_exit)
        )
        all_clauses.extend(self._profile_method("stays_on_exit", self._stays_on_exit))
        return all_clauses

    def _movement_rules_local(self):
        agent_var = self.ctx.agent_var
        neighbor_map = self.ctx.neighbor_map
        valid_positions = self.ctx.valid_positions

        for agent, _ in self.ctx.agents:
            c = agent.color
            for t in range(self.T_MAX):
                t1 = t + 1
                for x, y in valid_positions:
                    n_pos = neighbor_map[x, y]

                    # Forward: if at (x,y,t), must be at some neighbor at t+1
                    yield [-agent_var[c, x, y, t]] + [
                        agent_var[c, nx, ny, t1] for nx, ny in n_pos
                    ]
                    # Backward: if at (x,y,t+1), must have been at some neighbor at t
                    yield [-agent_var[c, x, y, t1]] + [
                        agent_var[c, nx, ny, t] for nx, ny in n_pos
                    ]
                    # Inline uniqueness: pairwise exclusion on neighbors at t+1
                    n = len(n_pos)
                    for i in range(n):
                        x1, y1 = n_pos[i]
                        v1 = -agent_var[c, x1, y1, t1]
                        for j in range(i + 1, n):
                            x2, y2 = n_pos[j]
                            yield [v1, -agent_var[c, x2, y2, t1]]

    def _movement_rules_global(self):
        agent_var = self.ctx.agent_var
        neighbor_map = self.ctx.neighbor_map
        valid_positions = self.ctx.valid_positions

        for agent, _ in self.ctx.agents:
            c = agent.color
            for t in range(self.T_MAX):
                t1 = t + 1
                for x, y in valid_positions:
                    n_pos = neighbor_map[x, y]
                    yield [-agent_var[c, x, y, t]] + [
                        agent_var[c, nx, ny, t1] for nx, ny in n_pos
                    ]

    def _unique_position(self):
        agent_var = self.ctx.agent_var
        all_positions = self.ctx.all_positions
        n = len(all_positions)

        for agent, _ in self.ctx.agents:
            c = agent.color
            for t in range(1, self.T_MAX + 1):
                for i in range(n):
                    x1, y1 = all_positions[i]
                    v1 = -agent_var[c, x1, y1, t]
                    for j in range(i + 1, n):
                        x2, y2 = all_positions[j]
                        yield [v1, -agent_var[c, x2, y2, t]]

    def _no_overlap(self):
        agent_var = self.ctx.agent_var
        agents = self.ctx.agents
        all_positions = self.ctx.all_positions
        n_agents = len(agents)

        for i in range(n_agents):
            c1 = agents[i][0].color
            for j in range(i + 1, n_agents):
                c2 = agents[j][0].color
                for t in range(self.T_MAX + 1):
                    t1 = t + 1
                    for x, y in all_positions:
                        v1_t = agent_var[c1, x, y, t]
                        v2_t = agent_var[c2, x, y, t]
                        yield [-v1_t, -v2_t]
                        yield [-agent_var[c1, x, y, t1], -v2_t]
                        yield [-v1_t, -agent_var[c2, x, y, t1]]

    def _must_be_on_exit(self):
        agent_var = self.ctx.agent_var
        T = self.T_MAX

        for x, y in self.ctx.exits:
            yield [agent_var[agent.color, x, y, T] for agent, _ in self.ctx.agents]

    def _stays_on_exit(self):
        agent_var = self.ctx.agent_var

        for agent, _ in self.ctx.agents:
            c = agent.color
            for t in range(self.T_MAX):
                t1 = t + 1
                for x, y in self.ctx.exits:
                    yield [-agent_var[c, x, y, t], agent_var[c, x, y, t1]]
