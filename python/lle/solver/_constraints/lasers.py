from collections.abc import Iterator

from lle.solver.variable_factory import VariableFactory
from lle.tiles import Laser, LaserSource

from .base import ConstraintContext, ConstraintGenerator
from .utils import equals, implies


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
                # yield [-agent_var, -laser_var]
                yield implies(self.var.agent(agent, x, y, t), -self.var.laser(laser.laser_id, x, y, t))

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

        ## Special cases
        ### First tile of the beam & no agent can reach
        If the tile if the first of its beam and no agent can reach the position, then the tile is active: (active_c(l, x, y, t)).

        ### First laser tile of the beam
        For the first laser tile of a beam (accessible by other agents), there is no previous tile and the general-case formula simplifies to

            active_c(l, x, y, t) = ¬agent(c, x, y, t),

        i.e. the laser is active if there is no agent on it. In CNF:

            (¬active_c(l, x, y, t) ∨ ¬agent(c, x, y, t)) ∧ (active_c(l, x, y, t) ∨ agent(c, x, y, t)).

        ### The agent can not reach the position
        For (x, y) positions that we know the agent cannot reach at step t, we know that agent(c, x, y, t) is False.
        Therefore, the formula becomes

            active_c(l, x, y, t) = active_c(l, prev_x, prev_y, t).

        """
        # iterate on every laser tile of the environment (i.e. every beam tile)
        for laser in self.lasers:
            x, y = laser.pos
            prev = self.ctx.get_prev_beam(x, y, laser.laser_id)
            agents_in_reach = [a for a in range(self.n_agents) if a == laser.agent_id and (x, y) in self.reachable_positions(t, a)]
            active = self.var.laser(laser.laser_id, x, y, t)
            # General case
            match (prev, agents_in_reach):
                case None, []:  # First tile & no agent in reach
                    yield [active]
                case None, [*agents]:  # First tile and some agents in reach
                    for a in agents:
                        agent = self.var.agent(a, x, y, t)
                        yield from equals(active, -agent)
                case ((prev_x, prev_y), []):  # Subsequent tile, no agent in reach
                    prev_active = self.var.laser(laser.laser_id, prev_x, prev_y, t)
                    yield from equals(active, prev_active)
                case ((prev_x, prev_y), [*agents]):  # General case
                    prev_active = self.var.laser(laser.laser_id, prev_x, prev_y, t)
                    # Encode: active = prev_active AND NOT(any agent on tile)
                    # Clauses:
                    #   active -> prev_active
                    #   active -> NOT agent_i for each agent i:   [-active, -agent_i]            (N clauses)
                    #   prev_active AND NOT(any agent) -> active: [-prev_active, *agents, active] (1 clause)
                    agent_vars = [self.var.agent(a, x, y, t) for a in agents]
                    # yield implies(active, prev_active)
                    yield [-active, prev_active]
                    yield from ([-active, -agent_var] for agent_var in agent_vars)
                    yield [-prev_active, *agent_vars, active]
                case other:
                    raise ValueError(f"There should be no other possible case but got: {other}")


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
