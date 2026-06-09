"""Universal cooperation properties of a world, proven via SAT/UNSAT.

Where :mod:`lle.cooperation.analyser` characterises cooperation *a posteriori*
(it replays one concrete trajectory and reports which ``(helper, beneficiary)``
help events occurred in *that* run), :func:`characterize` answers the dual,
**universal** question:

    *"does every valid plan of length ≤ t force agent ``helper`` to help agent
    ``beneficiary``?"*

It does so without ever enumerating trajectories, by proving — via the
unsatisfiability of an augmented SAT formula — that no independent counter-plan
exists.

# Key idea: "must depend" ⟺ "no independent counter-plan exists"

``depends(beneficiary, helper, t)`` — *"every valid plan of length ≤ t requires
``helper`` to help ``beneficiary``"* — is, by definition, the negation of an
existential statement::

    depends(b, h, t)  ⟺  ¬∃ τ ≤ t : ∃ valid plan of length τ in which
                                     `h` never helps `b`

So, per ordered pair ``(b, h)``, the only quantity to compute is the
*independence threshold*: the shortest plan length ``τ`` at which a plan exists
that does **not** require ``h`` to help ``b``.  Once known, ``depends(b, h, t)``
is simply ``t < independence_threshold(b, h)``.  No monotonicity assumption is
needed: this min-based definition is correct whether or not independent
solutions form an upward-closed set of lengths.

The dependency itself is tracked by the ``depends_on(beneficiary, helper)`` SAT
variable produced by the Rust :class:`~lle.solver.constraints.ClauseGenerator`
(``coop_clauses`` / ``finalize_depends_on``).  To ask *"is there a plan of this
length in which ``h`` does not help ``b``?"* we add ``¬depends_on(b, h)`` as a
**solver assumption** — a transient hypothesis that does not pollute the
accumulated clause set — which lets one incremental ``Minisat22`` instance per
horizon serve every pair.

See ``plan-cooperation.md`` for the full design rationale.
"""

from __future__ import annotations

from dataclasses import dataclass

from pysat.solvers import Minisat22

from ..solver.constraints import ClauseGenerator
from ..types import AgentId
from ..world import World


@dataclass(frozen=True)
class WorldCharacterization:
    """Universal cooperation properties of a world over plans of length ≤ ``t_max``.

    Obtained from :func:`characterize`.  Query it with :meth:`depends` (per ordered
    agent pair) and :meth:`is_independent` (global).
    """

    t_max: int
    """The largest plan length that was probed."""
    solution_lower_bound: int
    """A cheap admissible lower bound on the length of any valid plan."""
    first_solvable_length: int | None
    """The length of the shortest valid plan within ``[solution_lower_bound, t_max]``,
    or ``None`` if the world is unsolvable within that range."""
    independence_threshold: dict[tuple[AgentId, AgentId], int | None]
    """``(beneficiary, helper)`` -> shortest plan length at which a solution that does
    **not** require that dependency exists, or ``None`` if every solvable length
    ≤ ``t_max`` requires it."""
    fully_independent_threshold: int | None
    """Shortest plan length at which a fully cooperation-free plan (no help at all,
    by anyone, for anyone) exists, or ``None`` if none exists ≤ ``t_max``."""
    mutual_free_threshold: int | None
    """Shortest plan length at which a plan with *no mutual pair* exists, or ``None`` if every
    solvable plan ≤ ``t_max`` contains at least one pair (a, b) where both a helps b and b
    helps a.  Use :meth:`requires_mutual` to query this threshold."""
    chain_free_threshold: int | None
    """Shortest plan length at which a plan with *no temporal chain* exists (i.e. no triple
    (a, b, c) where a helped b strictly before b helped c), or ``None`` if no such plan exists
    ≤ ``t_max``.  Use :meth:`requires_chain` to query this threshold."""

    def depends(self, beneficiary: AgentId, helper: AgentId, t: int) -> bool:
        """Whether every valid plan of length ≤ ``t`` forces ``helper`` to help ``beneficiary``.

        Returns ``False`` when no valid plan of length ≤ ``t`` exists at all (there
        is nothing to "require"); use :attr:`first_solvable_length` to distinguish
        "provably independent" from "not even solvable".
        """
        if self.first_solvable_length is None or t < self.first_solvable_length:
            return False  # vacuous: no plan of length <= t exists at all
        if (beneficiary, helper) not in self.independence_threshold:
            return False  # not a tracked ordered pair (e.g. beneficiary == helper)
        threshold = self.independence_threshold[(beneficiary, helper)]
        return threshold is None or t < threshold

    def is_independent(self, t: int) -> bool:
        """Whether a fully cooperation-free plan of length ≤ ``t`` exists."""
        return self.fully_independent_threshold is not None and self.fully_independent_threshold <= t

    def requires_mutual(self, t: int) -> bool:
        """Whether every valid plan of length ≤ ``t`` contains a mutual pair.

        A *mutual pair* is an unordered pair ``{a, b}`` where both ``a helps b`` and
        ``b helps a`` in the plan.  Returns ``False`` when no valid plan of length ≤ ``t``
        exists at all (use :attr:`first_solvable_length` to distinguish "provably non-mutual"
        from "unsolvable").
        """
        if self.first_solvable_length is None or t < self.first_solvable_length:
            return False
        return self.mutual_free_threshold is None or t < self.mutual_free_threshold

    def requires_chain(self, t: int) -> bool:
        """Whether every valid plan of length ≤ ``t`` contains a temporal cooperation chain.

        A *chain* is a triple ``(a, b, c)`` of distinct agents where ``a`` helped ``b`` at
        some time step *strictly before* ``b`` helped ``c``.  Returns ``False`` when no valid
        plan of length ≤ ``t`` exists at all.
        """
        if self.first_solvable_length is None or t < self.first_solvable_length:
            return False
        return self.chain_free_threshold is None or t < self.chain_free_threshold


def characterize(world: World, t_max: int) -> WorldCharacterization:
    """Compute the universal cooperation properties of ``world`` for plans of length ≤ ``t_max``.

    For every ordered agent pair ``(beneficiary, helper)``, this finds the shortest
    plan length at which the ``helper``-helps-``beneficiary`` dependency stops being
    mandatory (its *independence threshold*).  It also computes four global thresholds:
    the first plan length at which a fully-independent, mutual-free, and chain-free plan
    exists respectively.

    The result is queried with :meth:`WorldCharacterization.depends` and
    :meth:`WorldCharacterization.is_independent`.

    # Algorithm

    A single incremental loop over horizons ``τ ∈ [solution_lower_bound, t_max]``:
    base, cooperation, and ``depends_on``-definition clauses are (re)assembled per
    horizon and solved once; then, reusing that same solver instance, each still
    unresolved pair is probed with the transient assumption ``¬depends_on(b, h)``.
    The first horizon at which such a probe succeeds is that pair's independence
    threshold.

    # Edge cases

    - **No plan exists within ``[solution_lower_bound, τ]``**: that horizon is
      skipped (nothing to characterise); :attr:`~WorldCharacterization.first_solvable_length`
      records the first solvable horizon, if any.
    - **``depends_on(b, h)`` is never created** (``helper``'s laser can never
      intersect ``beneficiary``'s reachable area within the horizon, e.g. agents
      that never interact): the pair is trivially independent and its threshold is
      recorded as the current (smallest solvable) horizon.
    - **Fewer than 2 agents**: there are no ordered pairs, so
      :attr:`~WorldCharacterization.independence_threshold` is empty and
      :meth:`~WorldCharacterization.depends` is vacuously ``False``.

    Args:
        world: The world to characterise.
        t_max: The largest plan length to probe (inclusive).

    Returns:
        A :class:`WorldCharacterization` summarising the dependency thresholds.
    """
    n = world.n_agents
    gen = ClauseGenerator(world, t_max)
    t_min = max(gen.solution_lower_bound, 0)

    pairs = [(b, h) for b in range(n) for h in range(n) if b != h]
    threshold: dict[tuple[AgentId, AgentId], int | None] = {p: None for p in pairs}
    unresolved: set[tuple[AgentId, AgentId]] = set(pairs)
    first_solvable_length: int | None = None
    fully_independent_threshold: int | None = None
    mutual_free_threshold: int | None = None
    chain_free_threshold: int | None = None

    if t_min <= t_max:
        # Clauses for horizons [0, t_min) are needed by every later solver instance.
        clauses: list[list[int]] = []
        for t in range(t_min):
            clauses.extend(gen.generate(t))
            clauses.extend(gen.coop_clauses(t))
            clauses.extend(gen.chain_clauses(t))  # must follow coop_clauses(t)

        for t in range(t_min, t_max + 1):
            clauses.extend(gen.generate(t))
            clauses.extend(gen.coop_clauses(t))
            clauses.extend(gen.chain_clauses(t))

            # Horizon-specific definitions: depends_on, mutual, chain.
            # finalize_mutual must be called after finalize_depends_on (reads dep vars).
            finalize_dep = gen.finalize_depends_on(t)
            finalize_mut = gen.finalize_mutual(t)
            finalize_chn = gen.finalize_chain(t)

            with Minisat22(bootstrap_with=clauses + finalize_dep + finalize_mut + finalize_chn) as solver:
                solver.append_formula(gen.objective(t))
                if not solver.solve():
                    continue  # unsolvable at this length
                if first_solvable_length is None:
                    first_solvable_length = t

                # --- Per-pair independence probes ---
                for b, h in list(unresolved):
                    lit = gen.depends_on_lit(b, h)
                    if lit is None:
                        threshold[(b, h)] = t
                        unresolved.discard((b, h))
                    elif solver.solve(assumptions=[-lit]):
                        threshold[(b, h)] = t
                        unresolved.discard((b, h))

                # --- Fully-independent probe ---
                if fully_independent_threshold is None:
                    neg = [-lit for b, h in pairs if (lit := gen.depends_on_lit(b, h)) is not None]
                    if solver.solve(assumptions=neg):
                        fully_independent_threshold = t

                # --- Mutual-free probe: is there a plan with no mutual pair? ---
                # Assume ¬mutual(a,b) for every pair that has a mutual variable.
                # If SAT, a mutual-free plan exists; if UNSAT, mutual is now mandatory.
                if mutual_free_threshold is None:
                    neg_mut = [-m for a in range(n) for b in range(a + 1, n) if (m := gen.mutual_lit(a, b)) is not None]
                    if solver.solve(assumptions=neg_mut):
                        mutual_free_threshold = t

                # --- Chain-free probe: is there a plan with no temporal chain? ---
                if chain_free_threshold is None:
                    neg_chn = [
                        -ch
                        for a in range(n)
                        for b in range(n)
                        for c in range(n)
                        if a != b and b != c and a != c and (ch := gen.chain_lit(a, b, c)) is not None
                    ]
                    if solver.solve(assumptions=neg_chn):
                        chain_free_threshold = t

            if (
                not unresolved
                and fully_independent_threshold is not None
                and mutual_free_threshold is not None
                and chain_free_threshold is not None
            ):
                break

    return WorldCharacterization(
        t_max=t_max,
        solution_lower_bound=t_min,
        first_solvable_length=first_solvable_length,
        independence_threshold=threshold,
        fully_independent_threshold=fully_independent_threshold,
        mutual_free_threshold=mutual_free_threshold,
        chain_free_threshold=chain_free_threshold,
    )
