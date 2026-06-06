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
        # `_beam_activation` must run first: it populates `self._active_lit`, the per-tile
        # activation-literal map that `_no_step_on_active_laser` reads.
        return [
            *self._beam_activation(t),
            *self._no_step_on_active_laser(t),
        ]

    def _no_step_on_active_laser(self, t: int):
        r"""
        Agents cannot step on an active laser beam of another colour.

        Formula:
        Given agent a of colour c and laser l of colour != c, if a_c is in (x, y), then l_c(x, y) must be off.
        - agent(a, x, y, t) -> ¬laser(l, x, y, t)

        When the beam tile is *constant active* (no same-colour agent can ever block it at time
        `t`), there is no laser variable to reference and the implication collapses to the unit
        clause `¬agent(a, x, y, t)`: the agent simply cannot be there.
        """
        # `None` for the strict subclass, which defines laser variables via beam propagation
        # and never folds constant-active tiles.
        active_lit = getattr(self, "_active_lit", None)
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
                agent_var = self.var.agent(agent, x, y, t)
                if active_lit is None:
                    yield [-agent_var, -self.var.laser(laser.laser_id, x, y, t)]
                    continue
                lit = active_lit.get((laser.laser_id, x, y))
                if lit is None:
                    yield [-agent_var]  # constant-active beam tile
                else:
                    yield [-agent_var, -lit]

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

        which splits down to the three following clauses:
        - active_c(l, x, y, t) -> active_c(l, prev_x, prev_y, t)
        - active_c(l, x, y, t) -> ¬agent(c, x, y, t)
        - active_c(l, x, y, t) ∨ ¬active_c(l, prev_x, prev_y, t) ∨ agent(c, x, y, t), i.e. if (prev and ¬agent), then active

        ## Special case: first tile of beam
        For the first laser tile of a beam, there is no previous tile and the general-case formula simplifies to

            active_c(l, x, y, t) = ¬agent(c, x, y, t).

        ## Unblockable tiles (constant folding)
        A beam tile can only be switched off by a same-colour agent standing on it. If the
        blocking agent cannot reach a tile at time `t` (it is not in
        `reachable_positions(t, source.agent_id)`), then that tile carries no agent term: its
        activation is exactly that of the previous beam tile.

        We exploit this to fold constant-active tiles away entirely. Walking a beam from its
        source, while no upstream tile is blockable the beam is unconditionally active, so we
        allocate no variable and emit no clause; once past the first blockable tile, an
        unblockable tile simply *aliases* the upstream variable (still no new variable, no
        clause). A variable and its defining clauses are introduced only for blockable tiles.
        The resulting `self._active_lit` maps each beam tile that has a variable to that literal;
        tiles absent from the map are constant active. Since 87–99% of beam-tile-timesteps on the
        canonical levels are unblockable, this removes the bulk of the laser variables and
        clauses while remaining logically equivalent.
        """
        active_lit: dict[tuple[int, int, int], int] = {}
        for source, full_path in self.ctx.laser_paths.items():
            blockable = self.reachable_positions(t, source.agent_id)
            prev_active = None  # `None` means the upstream beam is constant active.
            for x, y in full_path:
                if (x, y) in blockable:
                    agent = self.var.agent(source.agent_id, x, y, t)
                    active = self.var.laser(source.laser_id, x, y, t)
                    if prev_active is None:
                        # First blockable tile: active iff no blocking agent stands on it.
                        yield from equals(active, -agent)
                    else:
                        # active = (prev_active ^ ¬agent)
                        #  active -> prev_active ; active -> ¬agent ; prev_active ∧ ¬agent -> active
                        yield implies(active, prev_active)
                        yield implies(active, -agent)
                        yield [-prev_active, agent, active]
                    prev_active = active
                    active_lit[source.laser_id, x, y] = active
                elif prev_active is not None:
                    # Unblockable tile downstream of a blockable one: aliases the upstream literal.
                    active_lit[source.laser_id, x, y] = prev_active
                # else: constant-active tile — no variable, no clause, absent from the map.
        self._active_lit = active_lit


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

    def _laser_edges(self, source: LaserSource) -> Iterator[tuple[tuple[int, int], tuple[int, int]]]:
        path = self.ctx.laser_paths[source]
        yield from zip(path, path[1:])

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
                for x, y in self.ctx.laser_paths[source]:
                    if (x, y) not in self.reachable_positions(t, agent):
                        continue
                    yield [-self.var.agent(agent, x, y, t)]
