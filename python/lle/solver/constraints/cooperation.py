"""Cooperation indicator constraints for the SAT solver.

This module introduces SAT variables that track laser blocking and inter-agent
cooperation events within the standard-laser-mode formula.  They serve two
purposes:

1. **Strict-cooperation check without StrictLaserConstraints**:
   ``no_blocking_clauses(t)`` yields unit clauses that prevent any same-colour
   agent from standing on their own laser beam at time ``t``.  Adding these
   clauses for every ``t`` in ``[0, t_max]`` is logically equivalent to using
   ``StrictLaserConstraints``: if the augmented formula is UNSAT (for all
   horizons), cooperation is strictly required within that range.

2. **Cooperation-level tracking and enforcement**: the variables
   ``laser_blocked``, ``coop_event``, and ``depends_on`` let the solver reason
   about *who helps whom*, enabling both post-solve extraction of the
   cooperation structure and pre-solve enforcement of a target level.

Variable semantics
------------------
``laser_blocked(laser_id, t)``
    True iff the same-colour agent stands on (and therefore blocks) laser
    ``laser_id``'s beam at time ``t``.

``coop_term(helper, beneficiary, laser_id, i, j, t)``
    Auxiliary variable encoding a concrete pair: helper at beam[i] **and**
    beneficiary at beam[j] (with ``i < j``, so helper is upstream).

``coop_event(helper, beneficiary, laser_id, t)``
    True iff at least one valid ``(i, j)`` pair exists at time ``t``, i.e.
    helper is actively blocking laser ``laser_id`` while beneficiary occupies
    a protected downstream position.

``depends_on(beneficiary, helper)``
    True iff helper helps beneficiary at *some* point across the full horizon.
    Defined by ``finalize_depends_on`` which must be called once, after all
    per-timestep clauses have been generated.

``mutual(a, b)``  (canonical ``a < b``)
    True iff ``depends_on(a, b)`` **and** ``depends_on(b, a)`` are both true.
    Created on demand by ``require_mutual``.
"""

from __future__ import annotations

from collections.abc import Iterator

from ..variable_factory import VariableFactory
from .constraint import ConstraintGenerator
from .context import ConstraintContext


class CooperationConstraints(ConstraintGenerator):
    """Cooperation tracking and enforcement constraints.

    Add an instance of this class to the generator list alongside
    ``LaserConstraints`` to get cooperation indicator variables in the model.

    The typical usage pattern is::

        coop = CooperationConstraints(var, ctx)

        # Incremental loop:
        for t in range(t_min, t_max + 1):
            clauses.extend(coop.generate(t))

        # Once, before solving each candidate horizon:
        finalize = list(coop.finalize_depends_on(t_end))

        # Optional level-specific enforcement:
        level_clauses = list(coop.require_mutual())

        with Minisat22(bootstrap_with=clauses + finalize + level_clauses) as s:
            ...
    """

    def __init__(self, var: VariableFactory, ctx: ConstraintContext):
        super().__init__(var, ctx)
        self.n_agents = ctx.world.n_agents

    def generate(self, t: int) -> list:
        return [
            *self._laser_blocked_definitions(t),
            *self._coop_event_definitions(t),
        ]

    def no_blocking_clauses(self, t: int):
        """Yield unit clauses forbidding every laser-blocking event at time ``t``.

        Adding these clauses for all ``t`` in ``[0, t_max]`` is semantically
        equivalent to ``StrictLaserConstraints``: the resulting formula is UNSAT
        iff cooperation is strictly required within the given horizon.
        """
        for source, _ in self.ctx.laser_paths.items():
            for x, y in self.ctx.reachable_laser_paths[source][t]:
                yield [-self.var.agent(source.agent_id, x, y, t)]

    def _laser_blocked_definitions(self, t: int):
        """Define ``laser_blocked(laser_id, t) ↔ ∃(x,y) blockable: agent(colour, x, y, t)``."""
        for source, _ in self.ctx.laser_paths.items():
            blockable = self.ctx.reachable_laser_paths[source][t]
            if not blockable:
                continue
            blocked_var = self.var.laser_blocked(source.laser_id, t)
            agent_vars = [self.var.agent(source.agent_id, x, y, t) for x, y in blockable]
            # blocked → some agent is at a blocking position
            yield [-blocked_var, *agent_vars]
            # each blocking-position agent → blocked
            for av in agent_vars:
                yield [-av, blocked_var]

    # ------------------------------------------------------------------
    # Cooperation-event indicators
    # ------------------------------------------------------------------

    def _coop_event_definitions(self, t: int) -> Iterator[list[int]]:
        r"""Define ``coop_event(helper, beneficiary, laser_id, t)``.

        For each ordered pair ``(i, j)`` with ``i < j`` along laser
        ``laser_id``'s beam where position ``path[i]`` is reachable by helper
        and ``path[j]`` is reachable by beneficiary:

        .. code-block::

            coop_term(h, b, l, i, j, t) = agent(h, path[i], t) ∧ agent(b, path[j], t)

        ``coop_event`` is the OR over all such terms.
        """
        for source, path in self.ctx.laser_paths.items():
            helper = source.agent_id
            for beneficiary in range(self.n_agents):
                if beneficiary == helper:
                    continue

                # Collect valid (blocker_pos, beneficiary_pos) index pairs.
                pairs: list[tuple[int, int, int, int, int, int]] = []
                for i, (xi, yi) in enumerate(path):
                    if (xi, yi) not in self.ctx.reachable_laser_paths[source][t]:
                        continue
                    for j in range(i + 1, len(path)):
                        xj, yj = path[j]
                        if (xj, yj) not in self.ctx.reachable_positions(t, beneficiary):
                            continue
                        pairs.append((i, xi, yi, j, xj, yj))

                if not pairs:
                    continue

                coop_var = self.var.coop_event(helper, beneficiary, source.laser_id, t)
                term_vars: list[int] = []
                for i, xi, yi, j, xj, yj in pairs:
                    blocker_av = self.var.agent(helper, xi, yi, t)
                    benef_av = self.var.agent(beneficiary, xj, yj, t)
                    term = self.var.coop_term(helper, beneficiary, source.laser_id, i, j, t)
                    term_vars.append(term)
                    # term ↔ blocker ∧ beneficiary_present
                    yield [-term, blocker_av]
                    yield [-term, benef_av]
                    yield [-blocker_av, -benef_av, term]
                    # term → coop_event (each witness implies the event)
                    yield [-term, coop_var]

                # coop_event → OR(terms)  [close the iff]
                yield [-coop_var, *term_vars]

    # ------------------------------------------------------------------
    # depends_on  (aggregated over the full horizon)
    # ------------------------------------------------------------------

    def finalize_depends_on(self, t_end: int) -> Iterator[list[int]]:
        """Define ``depends_on(beneficiary, helper)`` over the horizon ``[0, t_end]``.

        Call this *once per solver instance*, **after** all per-timestep clauses
        have been generated.  The resulting clauses should be passed to the
        solver but must **not** be appended to the accumulating clause list
        (because the definition changes with each candidate ``t_end``).

        ``depends_on(b, a) ↔ ∃ s (colour=a), ∃ t ≤ t_end: coop_event(a, b, s.id, t)``
        """
        for source, _ in self.ctx.laser_paths.items():
            helper = source.agent_id
            for beneficiary in range(self.n_agents):
                if beneficiary == helper:
                    continue

                coop_vars = [
                    self.var.coop_event(helper, beneficiary, source.laser_id, t)
                    for t in range(t_end + 1)
                    if self.var.exists("coop_event", helper, beneficiary, source.laser_id, t)
                ]
                if not coop_vars:
                    continue

                dep_var = self.var.depends_on(beneficiary, helper)
                # dep_var → OR(coop_vars)
                yield [-dep_var, *coop_vars]
                # each coop_event → dep_var
                for cv in coop_vars:
                    yield [-cv, dep_var]

    # ------------------------------------------------------------------
    # Cooperation-level enforcement constraints
    # ------------------------------------------------------------------
    # All ``require_*`` methods assume ``finalize_depends_on`` has already
    # been called so that the ``depends_on`` variables exist.
    # ------------------------------------------------------------------

    def require_cooperation(self):
        """Assert at least one blocking event: the plan is not independent.

        Equivalent to requiring the plan to have ``CooperationLevel ≥ COOPERATIVE``.
        """
        blocked_vars = [
            self.var.laser_blocked(source.laser_id, t)
            for source in self.ctx.laser_paths
            for t in range(self.ctx.t_max + 1)
            if self.var.exists("laser_blocked", source.laser_id, t)
        ]
        if blocked_vars:
            yield blocked_vars

    def require_asymmetric(self):
        """Assert at least one directed dependency edge: ∃(helper, beneficiary).

        Equivalent to requiring ``CooperationLevel ≥ ASYMMETRIC``.
        Yields an empty (UNSAT) clause when no cooperation event is achievable.
        """
        dep_vars = [
            self.var.depends_on(b, a)
            for a in range(self.n_agents)
            for b in range(self.n_agents)
            if a != b and self.var.exists("depends_on", b, a)
        ]
        if not dep_vars:
            yield []  # no cooperation achievable → impossible
            return
        yield dep_vars

    def require_mutual(self):
        """Assert at least one mutual pair: ∃(a < b): dep(a,b) ∧ dep(b,a).

        Introduces ``mutual(a, b)`` auxiliary variables for each eligible pair.
        Equivalent to requiring ``CooperationLevel ≥ MUTUAL``.
        Yields an empty (UNSAT) clause when no mutual pair is achievable.
        """
        mutual_vars: list[int] = []
        for a in range(self.n_agents):
            for b in range(a + 1, self.n_agents):
                dep_ab_exists = self.var.exists("depends_on", a, b)
                dep_ba_exists = self.var.exists("depends_on", b, a)
                if not dep_ab_exists or not dep_ba_exists:
                    continue
                m = self.var.mutual(a, b)
                dep_ab = self.var.depends_on(a, b)  # b helps a
                dep_ba = self.var.depends_on(b, a)  # a helps b
                # mutual(a,b) ↔ dep(a,b) ∧ dep(b,a)
                yield [-m, dep_ab]
                yield [-m, dep_ba]
                yield [-dep_ab, -dep_ba, m]
                mutual_vars.append(m)
        if not mutual_vars:
            yield []  # no mutual pair achievable → impossible
            return
        # At least one mutual pair must exist
        yield mutual_vars

    def require_distributed(self) -> Iterator[list[int]]:
        """Assert that some beneficiary depends on ≥ 2 distinct helpers.

        Equivalent to requiring ``CooperationLevel ≥ DISTRIBUTED``.
        Meaningful only with ≥ 3 agents (otherwise equivalent to ``require_mutual``).
        """
        has_two_vars: list[int] = []
        for b in range(self.n_agents):
            helpers = [a for a in range(self.n_agents) if a != b and self.var.exists("depends_on", b, a)]
            if len(helpers) < 2:
                continue
            # For each pair of helpers (a1, a2), create an auxiliary variable.
            pair_vars: list[int] = []
            for idx1, a1 in enumerate(helpers):
                for a2 in helpers[idx1 + 1 :]:
                    pair_var = self.var.pool.id(("two_helpers", b, a1, a2))
                    dep_a1 = self.var.depends_on(b, a1)
                    dep_a2 = self.var.depends_on(b, a2)
                    yield [-pair_var, dep_a1]
                    yield [-pair_var, dep_a2]
                    yield [-dep_a1, -dep_a2, pair_var]
                    pair_vars.append(pair_var)
            if not pair_vars:
                continue
            has_two = self.var.pool.id(("has_two_helpers", b))
            has_two_vars.append(has_two)
            # has_two ↔ OR(pair_vars)
            yield [-has_two, *pair_vars]
            for pv in pair_vars:
                yield [-pv, has_two]

        if has_two_vars:
            yield has_two_vars

    def require_fully_coupled(self) -> Iterator[list[int]]:
        """Assert that every pair (a, b) with a ≠ b satisfies depends_on(a, b).

        Equivalent to requiring ``CooperationLevel = FULLY_COUPLED``.
        Requires at least 2 agents; yields an empty (UNSAT) clause otherwise.
        """
        if self.n_agents < 2:
            yield []  # fully-coupled is meaningless / impossible for < 2 agents
            return
        for a in range(self.n_agents):
            for b in range(self.n_agents):
                if a == b:
                    continue
                if not self.var.exists("depends_on", a, b):
                    # depends_on(a, b) was never created → no coop path exists → UNSAT
                    yield []
                    return
                yield [self.var.depends_on(a, b)]

    # ------------------------------------------------------------------
    # Model extraction
    # ------------------------------------------------------------------

    def extract_dependency_edges(self, model: list[int]) -> set[tuple[int, int]]:
        """Return the set of ``(helper, beneficiary)`` edges active in ``model``.

        Reads ``depends_on`` variables from the SAT model.  Requires
        ``finalize_depends_on`` to have been called before solving.
        """
        edges: set[tuple[int, int]] = set()
        for lit in model:
            if lit <= 0:
                continue
            obj = self.var.key(lit)
            if obj is None or obj[0] != "depends_on":
                continue
            _, beneficiary, helper = obj
            edges.add((helper, beneficiary))
        return edges
