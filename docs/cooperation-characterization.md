# Cooperation characterization

This document describes how cooperation is characterized in LLE by the Python package in `python/lle/characterization/` and by the cooperation-aware SAT solve modes exposed through `lle.solver`.

All world-level properties are bounded by `t_max`: a world can require a property for short plans and stop requiring it when a longer independent or simpler plan becomes available.

## Core notion: help edges

Cooperation is defined through a temporal dependency graph built from a trajectory.

A directed help edge

```text
helper -> beneficiary at time t
```

exists when, at state index `t`:

1. a laser beam of colour `helper` is enabled;
2. agent `helper` stands on one of its own beam tiles, blocking that beam; and
3. another agent, `beneficiary`, stands on a tile of that same beam without dying.

The trajectory analyser records these events as `DependencyEdge(helper, beneficiary, t)` in `TemporalDependencyGraph`. The graph keeps both:

- temporal edges, including the time step; and
- flattened edges, where repeated occurrences of the same `(helper, beneficiary)` pair at different times are collapsed.

Trajectory-level predicates answer: “Does this particular trajectory exhibit this pattern?”

World-level predicates answer: “Is this pattern required by every solution within `t_max`?” They are implemented by comparing normal solvability against SAT modes that forbid a pattern.

## Solvable worlds

A world is solvable when `solver.solve(world, t_max)` returns a plan.

In `WorldCharacterizer` this is exposed as:

- `shortest_path`: shortest plan found in standard mode;
- `is_solvable`: `shortest_path is not None`.

Most cooperation predicates raise `NotSolvableError` when queried on an unsolvable world, because “requires cooperation” is only meaningful if a solution exists.

## Independent / no cooperation

![Independent world with no required cooperation](cooperation-independent.png)

This example has no lasers: both agents can reach exits without any help edge.

### Trajectory-level

A trajectory is independent when its temporal dependency graph has no help edges:

```python
TrajectoryProfile.is_independent == graph.is_empty
```

Equivalently, no agent ever needs another agent to block a laser for it.

### World-level

A world is independent within `t_max` when there exists at least one solution that avoids all cooperation:

```python
WorldCharacterizer.is_independent == (shortest_independent_path is not None)
```

`shortest_independent_path` is obtained with:

```python
solver.solve(world, t_max, mode="no-cooperation")
```

The `no-cooperation` solve mode forbids any non-owner agent from occupying a laser span, equivalent to treating every beam as permanently active for non-owner agents.

## Cooperative

![Cooperative world requiring at least one help edge](cooperation-cooperative.png)

This built-in level illustrates the broad cooperative category: some help is required, before asking whether the help is asymmetric, mutual, chained, or cyclic.

### Trajectory-level

A trajectory is cooperative when it contains at least one help edge:

```python
TrajectoryProfile.is_cooperative == not graph.is_empty
```

### World-level

A world is cooperative within `t_max` when:

1. it is solvable in standard mode; and
2. it has no independent solution within `t_max`.

In code:

```python
WorldCharacterizer.is_cooperative == (shortest_independent_path is None)
```

after checking that `shortest_path` exists.

This means “some cooperation is forced”, not necessarily a specific cooperation structure such as mutual, chained, or asymmetric help.

## Asymmetric cooperation

![Asymmetric cooperation world](cooperation-asymmetric.png)

In this example, agent `0` must block its own east-facing laser so agent `1` can cross; agent `0` is not helped in return.

### Trajectory-level

A help edge `a -> b` is asymmetric when the helper `a` is never helped by any other agent anywhere in the same trajectory.

`TemporalDependencyGraph.asymmetric_edges()` computes this on flattened edges:

```python
edges = graph.flattened_edges()
helped_agents = {beneficiary for _, beneficiary in edges}
asymmetric = {
    (helper, beneficiary)
    for helper, beneficiary in edges
    if helper not in helped_agents
}
```

A trajectory is asymmetric when this set is non-empty:

```python
TrajectoryProfile.is_asymmetric == graph.has_asymmetric_edge()
```

Examples:

- `0 -> 1` is asymmetric if agent `0` is never helped.
- `0 -> 1 -> 2` has asymmetric edge `0 -> 1`, because `0` is not helped.
- `0 -> 1` and `1 -> 0` is not asymmetric: both helpers are also helped.

### World-level

A world requires asymmetric cooperation within `t_max` when:

1. it is solvable;
2. the shortest standard solution exhibits asymmetric cooperation;
3. no independent solution exists; and
4. no solution exists under `mode="no-asymmetric"`.

The dedicated `no-asymmetric` mode is necessary because “not asymmetric” is broader than “not cooperative”. An independent plan is non-asymmetric, but a mutual plan is also non-asymmetric even though it is cooperative.

In SAT terms, `no-asymmetric` forbids every concrete help event `helper -> beneficiary` unless `helper` is helped by some other agent somewhere by the final horizon:

```text
beneficiary occupies helper's blocked beam at tau
    -> OR_k has_helped_by_time(k, helper, t)
```

If no incoming-help indicator can exist for `helper`, the clause degenerates into forbidding that outgoing help event altogether.

## Mutual cooperation

![Mutual cooperation world](cooperation-mutual.png)

This example requires a two-way dependency: each of the two agents must help the other at some point.

### Trajectory-level

A trajectory is mutual when two agents help each other at least once, ignoring exact times:

```python
edges = graph.flattened_edges()
TrajectoryProfile.is_mutual == any((b, a) in edges for a, b in edges)
```

Thus, `a -> b` and `b -> a` anywhere in the same trajectory is mutual.

### World-level

A world requires mutual cooperation within `t_max` when:

1. it is solvable;
2. the shortest standard solution is mutual; and
3. no solution exists under `mode="no-mutual"`.

The `no-mutual` SAT mode forbids every pair of agents from both helping each other. It defines `has_helped_by_time(a, b, τ)` for each generated time step `τ` as a monotone prefix indicator: once `a` has helped `b`, the indicator remains true at all later time steps. Mutuality is then checked only at the current solve horizon `t` by rejecting:

```text
has_helped_by_time(a, b, t) AND has_helped_by_time(b, a, t)
```

For a fixed-horizon solve, this is equivalent to checking the two indicators at the final horizon. The per-step variables are used to define the prefix relation incrementally and are also shared by the interdependence encoding.

## Chained cooperation

![Chained cooperation world](cooperation-chained.png)

This example illustrates an open chain of dependencies: one help event enables a later help event by another agent.

### Trajectory-level

A chain is a temporal directed **trail** of help edges whose timestamps never decrease.  A trail is
a walk where no directed edge `(helper, beneficiary)` is traversed twice at the same time step `t`.
Formally, each temporal triple `(helper, beneficiary, t)` may appear at most once.

```text
a -> b -> c -> ...
```

The chain length is the number of help edges, not the number of agents. A single help edge is not considered a chain; the minimum meaningful length is `2`.

`TrajectoryProfile.is_chained(length=2)` is true when `graph.longest_chain() >= length`.

The graph implementation allows:

- open chains, such as `a -> b -> c`;
- cycles, such as `a -> b -> a` or `a -> b -> c -> a`;
- lassos, such as `a -> b -> c -> d -> b`;
- simultaneous chains: `a -> b` and `b -> c` at the same time step count as a length-2 chain; and
- vertex revisits: the same agent may appear multiple times, provided no directed pair is reused.

Because the help graph at any single time step is a finite simple directed graph (no repeated edges
within one step), and edges at different time steps are always distinct temporal triples, every trail
is finite.

### World-level

A world requires chained cooperation of length at least `N` when:

1. it is solvable;
2. it has no independent solution;
3. the shortest standard solution contains a chain of length `>= N`; and
4. no solution exists under `mode=f"no-chain-{N}"`.

`N` must be at least `2`. The bare mode string `"no-chain"` is canonical shorthand for `"no-chain-2"`.

The solver enumerates all directed trails of exactly length `N` (vertex sequences with no repeated
directed pair) and tracks whether any of them is realized using walk-progress variables.  Forbidding
each length-`N` trail also forbids all longer chains, because every trail of length `> N` contains
a sub-trail of length exactly `N`.

## Interdependence / cyclic help

![Interdependent cyclic cooperation world](cooperation-interdependent.png)

This example illustrates a three-agent cyclic dependency, where the help relation closes back on itself.

### Trajectory-level

Interdependence is a temporal cycle in the dependency graph. A cycle of order `K` visits `K` distinct agents and returns to the start with non-decreasing timestamps:

```text
a -> b -> c -> a
```

`TrajectoryProfile.interdependence_order()` returns the largest temporal cycle order, or `0` if none exists.

`TrajectoryProfile.is_interdependent(n_agents=2)` is true when the largest temporal cycle order is at least `n_agents`.

### World-level

A world requires interdependence of order at least `N` when:

1. it is solvable;
2. it has no independent solution;
3. the shortest standard solution contains a temporal cycle of order `>= N`;
4. it requires chained cooperation of length `N`; and
5. no solution exists under `mode=f"no-interdependence-{N}"`.

`N` must be at least `2`. The bare mode string `"no-interdependence"` is canonical shorthand for `"no-interdependence-2"`.

The extra chained-cooperation check is a shortcut: every temporal cycle of order `N` is also a chain of length `N`, so if a length-`N` chain is avoidable, an order-`N` cycle is avoidable too.

## Shortcuts and equivalences

### Time-bound characterizations

All world-level characterizations are relative to `t_max`. A world may require cooperation for small `t_max` but become independent for larger `t_max` if a longer detour exists.

### Independent solutions shortcut stronger properties

If a known independent solution exists, then the world cannot require asymmetric, chained, or interdependent cooperation, because the independent solution avoids all help edges.

The implementation reuses `shortest_independent_path` when it has already been computed.

### No laser colours shortcut

If no laser source exists, no help edge can exist. Therefore:

- the world cannot require asymmetric cooperation;
- no chain is possible;
- no cycle/interdependence is possible.

For asymmetric characterization, `shortest_non_asymmetric_path` returns the standard shortest path immediately when `n_laser_colours == 0`, avoiding a `no-asymmetric` SAT call.

### Chain and interdependence upper bounds

Under trail semantics (no repeated directed pair at the same time step), chained cooperation has a
finite structural upper bound.  At a single time step `t`, the help graph is a simple directed graph
over at most `n_agents` nodes, which has at most `n_agents × (n_agents − 1)` directed edges.  Any
trail within that graph therefore has length at most `n_agents × (n_agents − 1)`.  Across multiple
time steps, the same directed pair `(helper, beneficiary)` may be reused at a different `t` (each
temporal triple is distinct), but the set of distinct triples is finite, so every trail is finite.

A practical tighter bound within a single time step is `(n_agents − 1) × n_lasers`, since help can
only occur across laser beams and there are at most `n_lasers` beam types.

Interdependence has a separate structural bound: only agents that own at least one laser can be helpers in help edges, so a cycle of order greater than the number of laser-owning agents cannot occur.

### Monotone cache shortcuts

For chain and interdependence queries, the implementation caches SAT results by requested length/order and uses monotonicity:

- If a non-chained solution exists for a smaller or equal length, then a non-chained solution also exists for any larger length. Avoiding short chains is stricter than avoiding long chains.
- If no non-chained solution exists for a larger or equal length, then no non-chained solution exists for any smaller length. Requiring a longer chain implies requiring all shorter thresholds.

The same monotone reasoning is used for interdependence orders.

### Mutual help, chains, and cycles

Mutual help between two agents, `a -> b` and `b -> a`, is a chain of length `2` when the two help events can be ordered with non-decreasing times, including simultaneous help:

```text
a -> b -> a
```

It is also a temporal cycle of order `2` when the two edges can be ordered with non-decreasing timestamps, which is the temporal rule used by interdependence.

Important distinctions:

- `is_mutual` is computed on flattened edges and does not itself require a time ordering.
- `is_chained(2)` requires a temporal walk of two edges with non-decreasing timestamps.
- `is_interdependent(2)` requires a temporal cycle of order `2` under non-decreasing timestamps.

### Chained cooperation and interdependence are separate

A temporal cycle of order `N` is also a chain of length `N`, but interdependence requires a cycle over distinct agents while chained cooperation also includes open walks, lassos, and repeated agents/lasers. The implementation still keeps the encodings separate because the two properties ask for different structures.

An open chain can be required without any cycle. For example, `a -> b -> c` is chained but not interdependent.

### Chained cooperation includes mutual help, but is broader

Mutual help is one way to obtain a chain of length `2`, but it is not the only way:

- mutual help, including simultaneous mutual help: `a -> b -> a`;
- open chain: `a -> b -> c`.

Therefore `is_chained(2)` can be true while `is_mutual` is false.

### Asymmetric cooperation versus no cooperation

`no-cooperation` cannot be used as a shortcut for asymmetric characterization except in the positive independent-solution case.

Reason:

- `no-cooperation` forbids all help edges;
- `no-asymmetric` only forbids help edges whose helper is never helped.

A cooperative mutual plan may satisfy `no-asymmetric`, even though it does not satisfy `no-cooperation`. Therefore a dedicated `no-asymmetric` solve mode is required to prove that asymmetric help is unavoidable.

### Asymmetric and mutual are trajectory-level opposites only for the same two-agent pair

For a two-agent trajectory containing a single mutual pair `a -> b` and `b -> a`, there are no asymmetric edges, because both helpers are helped.

With more agents or additional edges, mutual and asymmetric patterns can coexist in the same trajectory. For example:

```text
a -> b, b -> a, c -> a
```

The pair `a`/`b` is mutual, while `c -> a` is asymmetric if `c` is never helped.

## Public entry points

The main public functions are available from `lle.characterization` and re-exported at package level:

- `characterize(world, t_max="auto")` returns a lazy `WorldCharacterizer`;
- `is_cooperative(world, t_max="auto")`;
- `is_asymmetric(world, t_max="auto")`;
- `is_mutual(world, t_max="auto")`;
- `is_chained(world, t_max="auto", length=2)`.

Trajectory-level analysis is available through:

- `profile_trajectory(world, trajectory)`;
- `TrajectoryProfile`;
- `TemporalDependencyGraph`.

The low-level SAT solve modes are:

- `"standard"`;
- `"no-cooperation"`;
- `"no-asymmetric"`;
- `"no-mutual"`;
- `"no-chain"` / `"no-chain-N"`;
- `"no-interdependence"` / `"no-interdependence-N"`.
