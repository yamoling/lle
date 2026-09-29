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

> **Precondition.** Since v2.12, an agent's colour is independent of its id and several agents may
> share a colour. The definitions below identify a beam's colour with its single owning agent, so
> they only hold when every colour belongs to exactly one agent. `ClauseGenerator::new` enforces
> this and returns `SolverError::SharedColour` otherwise; generalizing the taxonomy to colour
> groups is deliberately out of scope.

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

This built-in level illustrates the broad cooperative category: some help is required, before asking whether the help is asymmetric, mutual, sequential, or cyclic.

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

This means “some cooperation is forced”, not necessarily a specific cooperation structure such as mutual, sequential, or asymmetric help.

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

## Sequential cooperation

![Sequential cooperation world](cooperation-sequential.png)

This example illustrates an open sequence of dependencies: one help event enables a later help event by another agent.

### Trajectory-level

A sequence is a temporal directed **trail** of help edges whose timestamps never decrease. A trail is
a walk where no directed edge `(helper, beneficiary)` is traversed twice at the same time step `t`.
Formally, each temporal triple `(helper, beneficiary, t)` may appear at most once.

```text
a -> b -> c -> ...
```

The sequence length is the number of help edges, not the number of agents. A single help edge is not considered a sequence; the minimum meaningful length is `2`.

`TrajectoryProfile.is_sequential(length=2)` is true when `graph.longest_sequence() >= length`.

The graph implementation allows:

- open sequences, such as `a -> b -> c`;
- cycles, such as `a -> b -> a` or `a -> b -> c -> a`;
- lassos, such as `a -> b -> c -> d -> b`;
- simultaneous sequences: `a -> b` and `b -> c` at the same time step count as a length-2 sequence; and
- vertex revisits: the same agent may appear multiple times, provided no temporal edge
  `(helper, beneficiary, t)` is reused.

Because the help graph at any single time step is a finite simple directed graph (no repeated edges
within one step), and edges at different time steps are always distinct temporal triples, every trail
is finite.

### World-level

A world requires sequential cooperation of length at least `N` when:

1. it is solvable;
2. it has no independent solution;
3. the shortest standard solution contains a sequence of length `>= N`; and
4. no solution exists under `mode=f"no-sequence-{N}"`.

`N` must be at least `2`. The bare mode string `"no-sequence"` is canonical shorthand for `"no-sequence-2"`.

The solver enumerates all directed trails of exactly length `N`; a trail may revisit agents but
cannot reuse a temporal edge `(helper, beneficiary, t)`. It emits one blocking SAT clause for each
candidate trail. Forbidding each length-`N` trail also forbids all longer sequences, because every
trail of length `> N` contains a sub-trail of length exactly `N`.

## Interdependence / cyclic help

![Interdependent cyclic cooperation world](cooperation-interdependent.png)

This example illustrates a three-agent cyclic dependency, where the help relation closes back on itself.

### Trajectory-level

Interdependence is an exact-support temporal closed trail in the dependency graph. A witness of order `K` visits exactly `K` distinct agents and returns to its start with non-decreasing timestamps. Agents and static arcs may repeat, provided a concrete temporal edge `(helper, beneficiary, time)` is not reused:

```text
a -> b -> a -> c -> a
```

`TrajectoryProfile.interdependence_order()` returns the largest realized support order, or `0` if none exists. `TrajectoryProfile.is_interdependent_exactly(n_agents=K)` checks one exact order, while the compatibility predicate `is_interdependent(n_agents=K)` remains a threshold query.

Exact orders are not monotone: an order-4 ring need not contain an order-3 closed trail.

### World-level

A world requires interdependence of order exactly `N` when:

1. it is solvable;
2. the shortest standard solution contains a temporal closed trail of exact order `N`; and
3. no solution exists under `mode=f"no-interdependence-{N}"`.

`N` must be at least `2`. The bare mode string `"no-interdependence"` is canonical shorthand for `"no-interdependence-2"`.

The solver mode blocks every candidate temporal closed trail with exactly `N` distinct agents. Trails of other exact orders remain allowed, and no monotone inference is valid across orders. An order-`N` closed trail is a sequence, but it can contain more than `N` edges because agents may repeat.

## Shortcuts and equivalences

### Time-bound characterizations

All world-level characterizations are relative to `t_max`. A world may require cooperation for small `t_max` but become independent for larger `t_max` if a longer detour exists.

### Independent solutions shortcut stronger properties

If a known independent solution exists, then the world cannot require asymmetric, sequential, or interdependent cooperation, because the independent solution avoids all help edges.

The implementation reuses `shortest_independent_path` when it has already been computed.

### No laser colours shortcut

If no laser source exists, no help edge can exist. Therefore:

- the world cannot require asymmetric cooperation;
- no sequence is possible;
- no cycle/interdependence is possible.

For asymmetric characterization, `shortest_non_asymmetric_path` returns the standard shortest path immediately when `n_laser_colours == 0`, avoiding a `no-asymmetric` SAT call.

### Sequence and interdependence upper bounds

Under trail semantics (no repeated directed pair at the same time step), sequential cooperation has a
finite structural upper bound. At a single time step `t`, the help graph is a simple directed graph
over at most `n_agents` nodes, which has at most `n_agents × (n_agents − 1)` directed edges. Any
trail within that graph therefore has length at most `n_agents × (n_agents − 1)`. Across multiple
time steps, the same directed pair `(helper, beneficiary)` may be reused at a different `t` (each
temporal triple is distinct), but the set of distinct triples is finite, so every trail is finite.

A practical tighter bound within a single time step is `(n_agents − 1) × n_lasers`, since help can
only occur across laser beams and there are at most `n_lasers` beam types.

Interdependence has a separate structural bound: only agents that own at least one laser can be helpers in help edges, so a cycle of order greater than the number of laser-owning agents cannot occur.

### Cache shortcuts

Sequence queries use monotonicity: avoiding shorter sequences is stricter than avoiding longer ones. Exact interdependence orders are different: each order is cached and proved independently because an order-`N + 1` closed trail need not contain any order-`N` closed trail.

### Mutual help, sequences, and cycles

Mutual help between two agents, `a -> b` and `b -> a`, is a sequence of length `2` when the two help events can be ordered with non-decreasing times, including simultaneous help:

```text
a -> b -> a
```

It is also a temporal closed trail of exact order `2` when the two edges can be ordered with non-decreasing timestamps, which is the temporal rule used by interdependence.

Important distinctions:

- `is_mutual` is computed on flattened edges and does not itself require a time ordering.
- `is_sequential(2)` requires a temporal walk of two edges with non-decreasing timestamps.
- `is_interdependent_exactly(2)` requires a temporal closed trail of exact order `2` under non-decreasing timestamps.

### Sequential cooperation and interdependence are separate

A temporal closed trail of exact order `N` is also a sequence, but it may have more than `N` edges because interdependence permits repeated agents and static arcs at later times. Sequential cooperation also includes open walks and lassos. The implementation keeps the encodings separate because the two properties ask for different structures.

An open sequence can be required without any cycle. For example, `a -> b -> c` is sequential but not interdependent.

### Sequential cooperation includes mutual help, but is broader

Mutual help is one way to obtain a sequence of length `2`, but it is not the only way:

- mutual help, including simultaneous mutual help: `a -> b -> a`;
- open sequence: `a -> b -> c`.

Therefore `is_sequential(2)` can be true while `is_mutual` is false.

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
