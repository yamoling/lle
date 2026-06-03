from collections.abc import Iterator

from lle.solver.variable_factory import VariableFactory
from lle.tiles import Laser, LaserSource

from .base import ConstraintContext, ConstraintGenerator


class LaserConstraints(ConstraintGenerator):
    """SAT constraints that model laser propagation and blocking."""

    def __init__(self, var: VariableFactory, ctx: ConstraintContext):
        super().__init__(var, ctx)
        self.lasers: list[Laser] = ctx.world.lasers
        self.sources: list[LaserSource] = ctx.world.laser_sources
        self.n_agents = ctx.world.n_agents

    def generate(self, t: int):
        return [
            *self._no_step_on_active_laser(t),
            *self._beam_activation(t),
        ]

    def _laser_path(self, source: LaserSource) -> list[tuple[int, int]]:
        """Return the source tile followed by all beam tiles for one laser source."""
        path = [source.pos]
        dx, dy = source.direction.delta
        x = source.pos[0] + dx
        y = source.pos[1] + dy
        while 0 <= x < self.world.height and 0 <= y < self.world.width and (x, y) not in self.ctx.walls:
            path.append((x, y))
            x, y = x + dx, y + dy
        return path

    def _laser_edges(self, source: LaserSource) -> Iterator[tuple[tuple[int, int], tuple[int, int]]]:
        path = self._laser_path(source)
        yield from zip(path, path[1:])

    def _no_step_on_active_laser(self, t: int):
        r"""
        Agents cannot step on an active laser beam of another colour.

        Formula:
        Given agent a of colour c and laser l of colour != c, if a_c is in (x, y), then l_c(x, y) must be off.
        - agent(a, x, y, t) -> ¬laser(l, x, y, t)
        """
        if t == 0:
            return
        for agent in range(self.n_agents):
            reachable_positions = self.reachable_positions(t, agent)
            for laser in self.lasers:
                # Ignore lasers that can be blocked by the agent
                if laser.agent_id == agent:
                    continue
                # Ignore lasers that are not reachable at time step t
                if laser.pos not in reachable_positions:
                    continue
                x, y = laser.pos
                print((agent, x, y, t), (laser.laser_id, x, y, t))
                agent_var = self.var.agent(agent, x, y, t)
                laser_var = self.var.laser(laser.laser_id, x, y, t)
                yield [-agent_var, -laser_var]

    def _beam_activation(self, t: int):
        r"""
        Laser beams is active from the source until blocked by a same-colour agent.

        # Formula
        Let us note:
            - t, the time step
            - l, the laser id
            - c the laser/agent colour
            - (x, y), the coordinate location
            - active_c(l, x, y, t), the variable determining whether laser tile of colour c at location (x, y) is active at time step t
            - agent(c, x, y, t), the variable determining whether agent of colour c is at location (x, y) at time step t

        We want to encode that

            active_c(l, x, y, t) = active_c(l, prev_x, prev_y, t) ∧ ¬agent(c, x, y, t),

        which, in CNF, translates to the three following clauses:
        1. (¬active_c(l, x, y, t) ∨ active_c(l, prev_x, prev_y, t)): if the previous tile is off, then the current tile is also off;
        2. (¬agent(c, x, y, t) ∨ ¬active_c(l, x, y, t)): if there is an agent on the current tile, then the beam is off
        3. (active_c(l, x, y, t) ∨ ¬active_c(l, prev_x, prev_y, t) ∨ agent(c, x, y, t)): if the beam is active, the previous tile must be active, and no agent must be on the current tile

        ## Special cases
        ### First tile of the beam & no agent can reach
        If the tile if the first of its beam and no agent can reach the position, then the tile is active: (active_c(l, x, y, t)).

        ### First laser tile of the beam
        For the first laser tile of a beam (accessible by other agents), there is no previous tile and the general-case formula simplifies to

            active_c(l, x, y, t) = ¬agent(c, x, y, t),

        i.e. the laser is active if there is no agent on it. In CNF:

            (¬active_c(l, x, y, t) ∨ ¬agent(c, x, y, t)) ∧ (active_c(l, x, y, t) ∨ agent(c, x, y, t)).

        ### The agent can not reach the position
        For (x, y) positions that we know the agent cannot reach at step t, we know that agent(c, x, y, t) is False and the formula therefore simplifies to:
        1. unchanged, since agent(c, x, y, t) does not influence the clause;
        2. (true), since ¬agent(c, x, y, t) is true;
        3. (active_c(l, x, y, t) ∨ ¬active_c(l, prev_x, prev_y, t)), since agent(c, x, y, t) is false, it is removed from the formula.




        # Main formula translation to CNF
        The initial formula is x = ¬y ∧ z (c.f. previous section: Formula). The process to translate it to CNF is the following:
        1. Express equivalence as two implications: (x → (y ∧ ¬z)) ∧ ((y ∧ ¬z) → x)
        2. Eliminate the first implication (A → B ≡ ¬A ∨ B): ¬x ∨ (y ∧ ¬z)
        3. Apply the distributive law: (¬x ∨ y) ∧ (¬x ∨ ¬z)
        4. Eliminate the second implication: ¬(y ∧ ¬z) ∨ x
        5. Apply De Morgan's laws: (¬y ∨ z) ∨ x ≡ x ∨ ¬y ∨ z
        6. Combine the clauses: (¬x ∨ y) ∧ (¬x ∨ ¬z) ∧ (x ∨ ¬y ∨ z)
        """
        for laser in self.lasers:
            x, y = laser.pos
            # Clause 1: if the previous tile is off, then the current tile is also off
            prev = self.ctx.get_prev_beam(x, y, laser.laser_id)
            agents_in_reach = [a for a in range(self.n_agents) if a == laser.agent_id and (x, y) in self.reachable_positions(t, a)]
            active = self.var.laser(laser.laser_id, x, y, t)
            agent = self.var.agent(laser.agent_id, x, y, t)
            # General case
            match (prev, agents_in_reach):
                case None, []:  # First tile & no agent in reach -> the laser is active
                    yield [active]
                case None, [*agents]:  # First tile and there are agents in reach (simplification )
                    for a in agents:
                        yield [-active, -self.var.agent(a, x, y, t)]
                case ((prev_x, prev_y), []):
                    prev_active = self.var.laser(laser.laser_id, prev_x, prev_y, t)
                    # Unreachable tile -> active if the previous tile is actuve
                    yield [-active, -prev_active]
                    yield [active, -prev_active]
                case ((x, y), [*agents]):
                    ...

            for agent in range(self.n_agents):
                if laser.agent_id != agent:
                    continue
                if laser.pos not in self.reachable_positions(t, agent):
                    continue
                # Clause 2: if there is an agent on the current tile, then the beam is off
                yield [-self.var.agent(agent, x, y, t), -self.var.laser(laser.laser_id, x, y, t)]
                if prev_laser is not None:
                    # Clause 3:
                    yield [self.var.laser(laser.laser_id, x, y, t), -prev_laser, self.var.agent(agent, x, y, t)]
                can_be_blocked = True
            if not can_be_blocked:
                # If no agent can block the laser, then it is on if the previous beam is on
                prev_pos = self.ctx.get_prev_beam(x, y, laser.laser_id)
                if prev_pos is None:
                    # No previous -> first laser tile of the beam -> on by default
                    yield [self.var.laser(laser.laser_id, *laser.pos, t)]
                else:
                    prev_pos = self.var.laser(laser.laser_id, *prev_pos, t)
                    # prev is on -> current is on
                    yield [prev_pos, self.var.laser(laser.laser_id, *laser.pos, t)]


class StrictLaserConstraints(LaserConstraints):
    """
    Variant of LaserConstraints where beam propagation does NOT stop on agents.
    It only stops at walls / bounds (same as base behavior except agent blocking).
    """

    def generate(self, t: int):
        return [
            *self._no_step_on_active_laser(t),
            *self._beam_propagation(t),
            *self.forbid_agent_walk_on_tile_with_laser_of_other_colour(t),
        ]

    def _beam_propagation(self, t: int):
        for source in self.sources:
            for (x, y), (nx, ny) in self._laser_edges(source):
                src_var = self.var.laser(source.laser_id, x, y, t)
                dst_var = self.var.laser(source.laser_id, nx, ny, t)
                yield [-src_var, dst_var]
                yield [src_var, -dst_var]

    def forbid_agent_walk_on_tile_with_laser_of_other_colour(self, t: int):
        for source in self.sources:
            for agent in range(self.n_agents):
                if agent == source.agent_id:
                    continue
                for x, y in self._laser_path(source):
                    if (x, y) not in self.reachable_positions_for_agent(t, agent):
                        continue
                    yield [-self.var.agent(agent, x, y, t)]
