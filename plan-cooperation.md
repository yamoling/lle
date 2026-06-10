# Plan: `characterize(world, t_max)` — universal cooperation properties

## Context

The existing `lle.cooperation` package (`analyser.py`, `graph.py`, `profile.py`)
characterises cooperation **a posteriori**: it replays one concrete trajectory
and reports which `(helper, beneficiary)` help events occurred in *that* run.

The user wants the dual, **universal** question: *"does every solution of
length ≤ t require agent `helper` to help agent `beneficiary`?"* — answered
without ever enumerating trajectories, by proving (via SAT/UNSAT) that no
counter-example plan exists. The proof-of-concept target is the level

```text
 .  . S0 S1 .  .
L0E . .  .  @  .
 .  . .  .  .  .
 .  . .  .  .  .
 X  X .  .  .  .
```

where, intuitively, `depends(agent1, agent0, t)` is `True` for `t ≤ 9` (every
plan that finishes in ≤ 9 steps forces agent 0 to block its laser for agent 1
to cross) and `False` for `t ≥ 10` (agent 1 can detour behind the wall once
there is enough time budget).

The deliverable is a function `characterize(world, t_max) -> CooperationProperties`
that computes, for every ordered agent pair `(beneficiary, helper)`, the
threshold horizon at which the dependency stops being mandatory, plus a small
query surface (`.depends(beneficiary, helper, t)`) to read it back out per
length `t ≤ t_max`.

## Key idea: "must depend" ⟺ "no independent counter-plan exists"

`depends(beneficiary, helper, t)` — *"every valid plan of length ≤ t requires
`helper` to help `beneficiary`"* — is, by definition, the **negation of an
existential** statement:

```
depends(b, h, t)  ⟺  ¬∃ τ ≤ t : ∃ valid plan π of length τ with
                                 ¬depends_on(b, h) true under π
```

So the only thing we must compute, for every ordered pair `(b, h)`, is

```
independence_threshold(b, h) = min { τ ∈ [lower_bound, t_max] :
                                       ∃ valid plan of length τ in which
                                       depends_on(b, h) is false }
                               (or None if no such τ ≤ t_max exists)
```

Once we have that single number per pair, `depends(b, h, t)` is simply
`t < independence_threshold(b, h)` (and `False` when no plan of length ≤ t
exists at all — see "Edge cases" below). **No monotonicity assumption is
needed**: this min-based definition is correct whether or not "independent
solutions" or "solutions" form an upward-closed set of lengths.

This is exactly the kind of query the *existing* `depends_on(beneficiary,
helper)` SAT variable from `lle.solver.constraints_old.cooperation
.CooperationConstraints` was built for — we don't need any new clause types,
only a way to *assert its negation* and ask the solver "is that satisfiable?".
The cleanest way to assert a transient negation without polluting the
accumulated clause set is a **solver assumption**
(`solver.solve(assumptions=[-depends_on(b, h)])`), which `pysat` solvers
support natively and which lets us reuse one incremental `Minisat22` instance
per horizon `τ` for *all* pairs.

## Algorithm

Single incremental loop over horizons `τ ∈ [solution_lower_bound, t_max]`,
mirroring the existing `_solve` / `solve_no_cooperation` pattern in
`python/lle/solver/solver.py` and `_solve_with_coop` in
`python/tests/test_cooperation_constraints.py`:

```python
ctx = ConstraintContext(world, t_max)
t_min = max(ctx.solution_lower_bound, 0)
var = VariableFactory()
coop = CooperationConstraints(var, ctx)
generators = [InitializationConstraints(var, ctx),
              MovementConstraints(var, ctx),
              LaserConstraints(var, ctx),
              coop]
objective = ObjectiveGenerator(var, ctx)

pairs = [(b, h) for b in range(n) for h in range(n) if b != h]
threshold = {p: None for p in pairs}     # independence_threshold
unresolved = set(pairs)
first_solvable_length = None
fully_independent_threshold = None

clauses = [c for gen in generators for t in range(t_min) for c in gen.generate(t)]
for t in range(t_min, t_max + 1):
    clauses.extend(c for gen in generators for c in gen.generate(t))
    finalize = list(coop.finalize_depends_on(t))
    with Minisat22(bootstrap_with=clauses + finalize) as solver:
        solver.append_formula(objective.generate(t))
        if not solver.solve():
            continue                                  # unsolvable at this length
        if first_solvable_length is None:
            first_solvable_length = t

        # --- per-pair independence probes (transient assumptions) ---
        for (b, h) in list(unresolved):
            if not var.exists("depends_on", b, h):
                # the dependency cannot even occur within this horizon
                # -> any plan of this length is trivially independent for (b, h)
                threshold[(b, h)] = t
                unresolved.discard((b, h))
                continue
            if solver.solve(assumptions=[-var.depends_on(b, h)]):
                threshold[(b, h)] = t
                unresolved.discard((b, h))

        # --- global "fully independent" probe (bonus, see below) ---
        if fully_independent_threshold is None:
            neg = [-var.depends_on(b, h) for (b, h) in pairs if var.exists("depends_on", b, h)]
            if solver.solve(assumptions=neg):
                fully_independent_threshold = t

    if not unresolved and fully_independent_threshold is not None:
        break

return WorldCharacterization(
    n_agents=n, t_max=t_max, solution_lower_bound=t_min,
    first_solvable_length=first_solvable_length,
    independence_threshold=threshold,
    fully_independent_threshold=fully_independent_threshold,
)
```

Notes:
- Reusing **one solver instance per horizon** for every pair (instead of
  rebuilding the whole formula per pair) keeps this an `O(t_max)`-solver-build
  algorithm; the `O(n²)` extra work per horizon is just cheap assumption-based
  `solve()` calls that reuse already-learned clauses.
- `var.exists("depends_on", b, h)` must be checked *before* calling
  `var.depends_on(b, h)` — calling the factory method unconditionally would
  silently *create* a fresh, unconstrained variable (since `IDPool.id` creates
  on first access), which would corrupt the assumption (the solver could set
  it to whatever is convenient, since no clause defines it).
- `unresolved` lets us stop probing a pair as soon as its threshold is found,
  and the outer loop can `break` early once everything of interest is resolved.

## The `fully_independent_threshold` bonus property

The prompt's framing — *"the map **becomes independent** for `t ≥ 10`"* — is
actually a slightly stronger, *global* statement than "every per-pair
dependency individually has a counter-example": it claims a **single** plan
exists that exhibits **no** cooperation event at all. That is exactly what
`solve_no_cooperation` / `no_blocking_clauses` already test for (and is
equivalent to `len(solve_no_cooperation(world, t_max))` when it returns a
plan). The extra assumption-based probe above (`neg = [-depends_on(b,h) for
all existing pairs]`) computes the same threshold "for free" inside the same
loop, which both gives `characterize` a self-contained global verdict and
provides an internal-consistency check (`fully_independent_threshold ==
len(solve_no_cooperation(world, t_max))`) that is worth asserting in tests.

## New data structure: `WorldCharacterization`

A frozen dataclass, in the spirit of `CooperationProfile`
(`python/lle/cooperation/profile.py`):

```python
@dataclass(frozen=True)
class WorldCharacterization:
    n_agents: int
    t_max: int
    solution_lower_bound: int
    first_solvable_length: int | None
    """The length of the shortest valid plan within [lower_bound, t_max], or None if unsolvable."""
    independence_threshold: dict[tuple[AgentId, AgentId], int | None]
    """(beneficiary, helper) -> shortest plan length at which an independent
    (w.r.t. that pair) solution exists, or None if every solvable length ≤ t_max
    requires the dependency."""
    fully_independent_threshold: int | None
    """Shortest plan length at which a fully-independent (no cooperation at all)
    solution exists, or None if none exists ≤ t_max."""

    def depends(self, beneficiary: AgentId, helper: AgentId, t: int) -> bool:
        """True iff every valid plan of length <= t forces `helper` to help `beneficiary`."""
        if self.first_solvable_length is None or t < self.first_solvable_length:
            return False  # vacuous: no plan of length <= t exists at all
        threshold = self.independence_threshold.get((beneficiary, helper))
        return threshold is None or t < threshold

    def is_independent(self, t: int) -> bool:
        """True iff a fully-cooperation-free plan of length <= t exists."""
        return self.fully_independent_threshold is not None and self.fully_independent_threshold <= t
```

## Edge cases & semantics decisions to record in docstrings

- **No plan exists at all within `[lower_bound, t]`**: `depends` returns
  `False` (there is nothing to "require"); `first_solvable_length` lets a
  caller distinguish "provably independent" from "not even solvable".
- **`depends_on(b, h)` never created** (agent `h`'s laser can never intersect
  agent `b`'s reachable area within the horizon, e.g. different colours that
  never interact): the pair is trivially independent — `independence_threshold[(b,h)]
  = first_solvable_length`-ish (handled by the `var.exists` branch above,
  which records the *current* `t`, i.e. the first horizon at which this branch
  is reached and a solution exists — by construction this is the smallest such
  `t`, since the loop starts at `t_min` and the "doesn't exist" condition is
  monotone: once `coop_event` becomes possible at some horizon it stays
  possible at every larger horizon).
- **`n_agents < 2`**: `pairs` is empty; `independence_threshold == {}`,
  `depends` is vacuously `False` for any pair query (there can be no pairs).

## Where this lives & how it's wired in

- New module: `python/lle/characterization.py`, exporting
  `characterize` and `WorldCharacterization`.
- Re-export both from `python/lle/cooperation/__init__.py` (alongside
  `CooperationProfile`, `analyse_cooperation`, …) and from the top-level
  `python/lle/__init__.py` `__all__`, mirroring how `CooperationProfile` /
  `analyse_cooperation` are surfaced today.
- Reuse, **without modification**:
  - `lle.solver.constraints_old.{ConstraintContext, CooperationConstraints,
    InitializationConstraints, MovementConstraints, LaserConstraints,
    ObjectiveGenerator}` for clause generation (the same pipeline already
    exercised by `_solve_with_coop` in `test_cooperation_constraints.py`),
  - `lle.solver.variable_factory.VariableFactory` for variable IDs / `exists`
    checks,
  - `pysat.solvers.Minisat22` for incremental solving with assumptions.
- **Important caveat**: this PoC deliberately builds on the *Python*
  `constraints_old` pipeline (which already has `depends_on` / `coop_event` /
  `laser_blocked`), not the new Rust `ConstraintGenerator` exposed in
  `lle.solver.constraints` — the Rust port has not yet ported the cooperation
  -tracking variables (only `no_blocking_clauses` made it across, see
  `src/solver/clauses.rs`). Porting `CooperationConstraints` to Rust is a
  natural follow-up once this Python PoC has validated the approach and the
  property semantics, but is out of scope here.

## Test plan

Add `python/tests/test_cooperation_characterization.py`:

1. Build the PoC level from a literal map string:
   ```python
   world = World("""
    .   .  S0  S1  .   .
   L0E  .   .   .  @   .
    .   .   .   .  .   .
    .   .   .   .  .   .
    X   X   .   .  .   .
   """)
   ```
2. `props = characterize(world, t_max=12)` (or similar — large enough to see
   the transition).
3. Assertions:
   - `props.depends(1, 0, 9) is True` — agent 1 needs agent 0's help up to
     length 9.
   - `props.depends(1, 0, 10) is False` and `props.depends(1, 0, 12) is False`
     — the dependency is no longer mandatory once enough time is available.
   - `props.depends(0, 1, t) is False` for all relevant `t` — agent 0 (the
     laser's own colour) never needs agent 1's help in this map.
   - `props.independence_threshold[(1, 0)] == 10`.
   - `props.is_independent(9) is False` and `props.is_independent(10) is True`.
   - Cross-check: `props.fully_independent_threshold == len(solve_no_cooperation(world, t_max=12))`.
4. Use the project's 60-second timeout convention for solver-backed tests
   (per `CLAUDE.md`).

## Verification

- `uv run pytest python/tests/test_cooperation_characterization.py -x` (after
  `maturin dev`).
- Sanity-run `characterize` interactively on `World.level(1..6)` to confirm it
  terminates quickly and produces internally consistent thresholds (e.g.
  `fully_independent_threshold` agreeing with `solve_no_cooperation`).
- `basedpyright` to type-check the new dataclass and function signatures.
