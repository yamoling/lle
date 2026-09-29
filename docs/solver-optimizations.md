# SAT Solver Optimizations

This document catalogues every optimization applied to the bounded-planning SAT encoding used by `lle.solver.solve`. Each optimization either reduces the number of SAT variables and clauses generated, improves the quality of the encoding, or shrinks the search space explored by the incremental solving loop.

## Summary

| Optimization | Category | Status |
---|---|---|
| Agent reachability map | Variable pruning | Complete |
| Exit-reachability filtering | Variable pruning | Complete |
| Relevant laser path | Variable pruning | Complete |
| Unblockable laser tile pruning | Variable pruning | Complete |
| Constant-active beam tile | Variable pruning | Complete |
| Start tile pruning | Variable pruning | Complete |
| Forced exit pruning | Variable pruning | Complete |
| First beam tile pruning | Variable pruning | Complete |
| At-most-one encoding selection | Clause encoding | Complete |
| Incremental clause buffering | Clause generation | Complete |
| Solution lower bound | Search | Complete |
| Trajectory symmetry breaking | Variable pruning | Not implemented |

---

## 1. Variable pruning through time-bounded reachability

The core idea behind all variable-pruning optimizations is that a SAT variable $\text{agent}(a, p, t)$ — "agent $a$ is at position $p$ at time $t$" — is only useful if it is satisfiable in at least one valid plan. Creating variables for positions that are provably unreachable wastes memory, increases clause counts, and enlarges the search space for the solver.

### 1.1 Agent reachability map

**Intuition.** An agent starts at a fixed position $s_a$ at $t = 0$. Since each action moves it by at most one cell, it can only be within a BFS ball of radius $t$ around $s_a$ at time $t$. Any position outside that ball can be discarded.

**Implementation.** Let $\text{reach}(a, t)$ be the set of positions agent $a$ can occupy at time $t$. This is computed incrementally:

$$\text{reach}(a, 0) = \{s_a\}$$

$$\text{reach}(a, t) = \bigcup_{p \in \text{reach}(a, t-1)} \text{succ}(p)$$

where $\text{succ}(p)$ is the set of positions reachable from $p$ in one step (including staying in place, excluding walls, voids, and out-of-bounds cells). This is a forward BFS seeded at $s_a$, evaluated incrementally as the time horizon grows.

Only positions in $\text{reach}(a, t)$ get an $\text{agent}(a, \cdot, t)$ variable.

### 1.2 Exit-reachability filtering

**Intuition.** For a plan of length $T$ (the current horizon), an agent that has not yet reached an exit at time $t$ must be able to reach an exit within $T - t$ remaining steps. Any position from which no exit is reachable within $T - t$ steps can be discarded.

**Implementation.** Let $d(p)$ be the shortest distance from position $p$ to the nearest exit, computed once by a backward BFS from all exits (ignoring lasers). Define:

$$\text{exit\_reach}(t) = \{ p \mid d(p) \leq T - t \}$$

This set shrinks as $t$ grows: at each step, exactly the positions at distance $T - t + 1$ fall out. It is therefore updated incrementally by removing one distance bucket per step rather than recomputing from scratch.

The actual relevant set used for variable creation is the intersection:

$$\text{relevant}(a, t) = \text{reach}(a, t) \cap \text{exit\_reach}(t)$$

Because both sets are stored as dense bitsets over the grid, the intersection is a word-at-a-time AND operation.

### 1.3 Start tile pruning

**Intuition.** At $t = 1$, agent $A$ cannot occupy the start position $s_B$ of any other agent $B \neq A$. This is a consequence of the no-following-conflict rule: since $B$ is at $s_B$ at $t = 0$, the implication $\text{agent}(A, s_B, 1) \Rightarrow \neg\text{agent}(B, s_B, 0)$ combined with the unit fact $\text{agent}(B, s_B, 0)$ forces $\neg\text{agent}(A, s_B, 1)$. The variable is always false and can be pruned.

**Implementation.** In the $t = 1$ update of $\text{relevant}(A, t)$, the start positions of all agents $B \neq A$ are explicitly removed. This single-step removal propagates into subsequent timesteps if the start position happens to be the only route between $A$'s starting neighbourhood and the rest of the grid, though typically the effect is local to $t = 1$.

### 1.4 Forced exit pruning

**Intuition.** When a world has exactly as many exits as agents, every exit must be occupied in any satisfying objective state: all agents must end on an exit, and no-overlap forbids two agents from sharing the same exit tile. If an exit can be reached by exactly one agent, then that agent is forced to use that exit. Since exits are absorbing, that same agent cannot stop on any other exit at any earlier or final timestep.

For example, in the benchmark layout below, agent 2 is the only agent that can reach exit $(4, 1)$ because the adjacent non-exit access is the first tile of laser colour 2. Therefore, exit positions $(4, 2)$ and $(4, 3)$ can be removed from agent 2's relevant-position map.

```text
 @  S0 S1 S2
L0E .  .  .
L1E .  .  .
L2E .  .  .
 @  X  X  X
```

**Implementation.** After the raw relevant-position map has been filtered by movement, exit reachability, first beam tiles, and unblockable laser tiles, each timestep scans the exit positions. If the number of exits equals the number of agents, and an exit is present in exactly one agent's relevant set, that exit is recorded as uniquely forced for that agent. Agents with exactly one uniquely forced exit then have every other exit removed from their relevant-position set for that timestep.

The optimization is conservative: it is disabled when the number of exits differs from the number of agents, and it does not prune an agent that is the sole candidate for multiple exits at the same timestep.

#### Actual gain

On the benchmark layout above with `t_max = 20`, standard solving mode, and no gems, the generated formula changed as follows:

| Metric | Before | After | Absolute reduction | Relative reduction |
|---|---:|---:|---:|---:|
| Variables | 1 230 | 1 169 | 61 | 4.96% |
| Clauses | 4 278 | 3 949 | 329 | 7.69% |

---

## 2. Variable pruning for laser beams

Laser beam constraints require a second family of variables $\text{laser}(l, p, t)$ — "beam $l$ is active at position $p$ at time $t$". These variables and their defining clauses are only useful when blocking is possible, i.e., when the owning agent can potentially reach a beam tile.

### 2.1 Relevant laser path

**Intuition.** Consider a beam owned by agent $c$ along a sequence of tiles $[p_0, p_1, \ldots, p_k]$. A tile $p_i$ is worth reasoning about at time $t$ only if:

1. Agent $c$ can reach $p_i$ at time $t$ — it can block the beam there, or
2. Some upstream tile $p_j$ ($j < i$) satisfies condition 1 — the beam can be blocked before $p_i$ — **and** some non-owner agent can reach $p_i$ at time $t$ — a cooperation scenario where the owner shields a downstream ally.

Tiles satisfying neither condition are always active regardless of agent actions. No laser variable is needed for them (see §2.2).

**Implementation.** Let $\text{owner}(l)$ denote the agent owning laser $l$. The relevant beam tiles are:

$$\text{rel\_beam}(l, t) = \bigl\{ p_i \in \text{path}(l) \mid p_i \in \text{relevant}(\text{owner}(l), t) \bigr\}\ \cup\ \bigl\{ p_i \mid \exists\, j < i,\ p_j \in \text{relevant}(\text{owner}(l), t),\ \exists\, a \neq \text{owner}(l),\ p_i \in \text{relevant}(a, t) \bigr\}$$

The beam tiles are scanned in order; a flag tracks whether any upstream tile is blockable by the owner, and downstream tiles are included only when that flag is set and some non-owner can reach them.

### 2.2 Unblockable laser tile pruning

**Intuition.** The relevant-position map and the relevant-laser-path map are mutually informative. If a non-owner agent $a$ can reach a downstream beam tile $p_i$ at time $t$, that position is only genuinely relevant if the beam owner can block the beam on some strictly upstream tile $p_j$ with $j < i$ at the same time step. If no such upstream block is possible, then $p_i$ is necessarily active for $a$ and $\text{agent}(a, p_i, t)$ is always false.

This is stronger than merely omitting a laser variable. Without this pruning, the SAT encoding may still create an agent-position variable for $\text{agent}(a, p_i, t)$ and later force it false with a constant-active unit clause (§2.3). Removing the position directly from $\text{relevant}(a, t)$ avoids creating that variable and also removes all clauses that would have mentioned it: exactly-one-position clauses, movement clauses, overlap clauses, following-conflict clauses, and laser-safety clauses.

The upstream block must be strict. A non-owner standing on $p_i$ cannot rely on the owner also blocking at $p_i$, because no-overlap forbids both agents from occupying the same tile at the same time. This is why the first beam tile of a foreign laser is always pruned (§2.4).

**Implementation.** After the raw $\text{relevant}(a, t)$ sets have been computed for all agents and filtered by exit reachability, each laser path is scanned in order. For a laser $l$ owned by $c$, the scan maintains a Boolean flag indicating whether a tile already seen upstream belongs to $\text{relevant}(c, t)$. Until that flag becomes true, every scanned beam tile is removed from $\text{relevant}(a, t)$ for all $a \neq c$.

The same pass also contributes to the relevant-laser-path cache: owner-reachable beam tiles are inserted immediately, and downstream candidate tiles are recorded once an upstream block exists. After all lasers have pruned positions, the downstream candidates are finalized against the fully pruned $\text{relevant}(a, t)$ sets. This two-phase finalization avoids order-dependent results when laser paths overlap or cross.

#### Actual gain
The gain is roughly 0.4% in clauses and 0.4% in number of variables for levels 5 and 6 of LLE.

### 2.3 Constant-active beam tile

**Intuition.** If a beam tile $p_i$ is not in $\text{rel\_beam}(l, t)$ — the owner cannot block the beam at or before $p_i$ — the beam is permanently active at $p_i$ for all valid agent configurations. The constraint on a non-owner agent $a$ simplifies to the unit clause $\neg\text{agent}(a, p_i, t)$, which the SAT solver propagates for free.

**Implementation.** During the `no_step_on_active_laser` pass, tiles absent from the laser variable map are treated as constant-active. For each non-owner agent $a$ whose relevant positions include such a tile, a unit clause $[-\text{agent}(a, p_i, t)]$ is emitted instead of a binary clause. With unblockable laser tile pruning (§2.2), many such agent-position variables are removed earlier and no unit clause is needed.

### 2.4 First beam tile pruning

**Intuition.** The first tile $p_0$ of any beam can only be blocked by the owning agent standing on $p_0$ itself. For a non-owner agent $a$ to be safely at $p_0$, the beam must be inactive, which requires the owner to also occupy $p_0$ — impossible by the no-overlap constraint. Therefore $p_0$ can be removed from $\text{relevant}(a, t)$ for every non-owner $a$ and every $t$.

The argument holds in both cases:
- If the owner can reach $p_0$: beam activation gives $\text{laser}(l, p_0, t) \leftrightarrow \neg\text{agent}(\text{owner}(l), p_0, t)$. The `no_step_on_active_laser` clause $\neg\text{agent}(a, p_0, t) \lor \neg\text{laser}(l, p_0, t)$ then yields $\neg\text{agent}(a, p_0, t) \lor \text{agent}(\text{owner}(l), p_0, t)$, which combined with the no-overlap clause forces $\neg\text{agent}(a, p_0, t)$.
- If the owner cannot reach $p_0$: the tile is constant-active and the unit clause $\neg\text{agent}(a, p_0, t)$ is generated directly (§2.2).

**Implementation.** At construction time, the first tile of every beam is collected for each laser source. For each agent $a$, the set of first beam tiles of lasers not owned by $a$ is pre-computed once. This set is subtracted from $\text{relevant}(a, t)$ at every timestep before any variable is allocated.

---

## 3. Clause encoding

### 3.1 At-most-one encoding selection

**Intuition.** The exactly-one-position constraint for agent $a$ at time $t$ requires an at-least-one disjunction and an at-most-one constraint over the $n = |\text{relevant}(a, t)|$ position variables. Two standard encodings exist:

- **Pairwise**: for every pair $(x_i, x_j)$, add $\neg x_i \lor \neg x_j$. This produces $\binom{n}{2}$ clauses and no auxiliary variables.
- **Sequential counter**: introduces $n - 1$ auxiliary variables $s_1, \ldots, s_{n-1}$ with implications $x_i \Rightarrow s_i$, $x_{i+1} \Rightarrow \neg s_i$, and $s_i \Rightarrow s_{i+1}$. This produces $O(n)$ clauses at the cost of $n - 1$ auxiliary variables.

Pairwise is optimal for small $n$: it uses fewer or equal clauses and no extra variables. The sequential counter only wins on clause count for $n \geq 6$, but at the price of $n - 1$ auxiliary variables that expand the variable space.

**Implementation.** The crossover threshold is set at $n = 5$. For $|\text{relevant}(a, t)| \leq 5$, the pairwise encoding is used. For larger sets, the sequential counter is used. This keeps the encoding compact for the common case where the reachable frontier is small near the start and end of the horizon.

---

## 4. Search efficiency

### 4.1 Incremental clause buffering

**Intuition.** The solver calls `generate(t)` for $t = t_{\min}, t_{\min} + 1, \ldots$ until a solution is found. The formula for horizon $t$ is a strict superset of the formula for $t - 1$: the world-enforcing clauses for steps $0 \ldots t-1$ are identical across calls; only the step-$t$ clauses and the objective differ. Recomputing the full formula at each call would waste time proportional to $t^2$.

**Implementation.** World-enforcing clauses are buffered per timestep in a vector $\text{buffer}[t]$, populated at most once. When `generate(t)$ is called, only the steps not yet buffered are computed. The formula returned is the concatenation of all buffered steps $0 \ldots t$, the objective clauses for horizon $t$, and any mode-specific clauses.

### 4.2 Solution lower bound

**Intuition.** The incremental loop iterates over horizons $t = 0, 1, \ldots, T_{\max}$. For small $t$, no agent can possibly reach an exit in time and the formula is trivially unsatisfiable, yet the solver still has to process it. Starting from a tighter lower bound avoids these wasted calls.

**Implementation.** At construction time, the shortest walkable distance from each agent's start to the nearest exit is read from the backward-BFS distance map. The lower bound is:

$$t_{\min} = \max_{a} \, d(s_a)$$

The incremental loop starts at $t_{\min}$ rather than $0$. Because the distance computation ignores lasers, the bound is admissible — it can only undercount the true minimum, never overcount.

---

## 5. Not yet implemented

### 5.1 Trajectory symmetry breaking

**Intuition.** Consider a single agent travelling from $A$ to $C$ in two steps via $B$. The action sequences $[\text{SOUTH}, \text{STAY}]$ and $[\text{STAY}, \text{SOUTH}]$ both produce the same time-wise position sequence $(A, B, C)$. Encoding both trajectories is redundant: the SAT solver may explore both and find the same solution twice.

More generally, for a fixed agent and a fixed sequence of visited positions $(p_0, p_1, \ldots, p_T)$, there may be multiple action sequences that realise it by permuting STAY steps. Only one canonical representative needs to be encoded.

When multiple agents are present, the redundancy is safe to exploit only for positions that no other agent can reach — i.e., positions that are "private" to one agent. At shared positions, different orderings can change interaction outcomes (overlap, following conflict) and cannot be freely merged.

**Why it is hard.** Identifying canonical representatives requires detecting STAY-equivalent trajectory classes and adding symmetry-breaking clauses that select exactly one representative from each class. These extra clauses interact with the no-overlap and no-following-conflict constraints in non-trivial ways. The general multi-agent case, where the set of private positions must be determined dynamically and per-pair, is complex to implement correctly.



# Others
## Prevent stay in irrelevant positions
In most cases, it is irrelevant for agents to remain in the same position. At first glance, the only reasons to remain in the same spot are:
- an exit has been reached, then staying is mandatory
- you are blocking a laser beam
- you are waiting for some agent to block a laser beam


As such, when computing the reachability map, we should take these three cases into account, and therefore not generate variables in positions that are "irrelevant".
