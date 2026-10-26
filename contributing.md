# Contributing Guidelines

Welcome to the LLE contributing guidelines.

## Styles & Standards

### Language

Everything should be documented in English with [Oxford spelling](https://en.wikipedia.org/wiki/Oxford_spelling).

From Wikipedia: "Oxford spelling uses the spelling ‑ize alongside ‑lyse: organization, realize, privatize and recognizable, rather than organisation, realise, privatise and recognisable – but analyse and paralyse. Words such as advise, advertise, improvise, surprise are spelled thus in all varieties of English, since ‑ise in them is not a suffix, but a part of an English or French root."

Additionally, words in "-our" such as "colour", "neighbour" or "behaviour" should be spelled with an "u" (e.g. "colour" not "color").

### Numbering

Number should be written in the international standard, i.e. numbers divided in groups of three separated by a blank space. The decimal point should be separated by a comma.
For instance:

- `1 000` reads as "one thousand"
- `1,000` reads as "one"
- `2 000.18` and `2 000,18` both read as "two thousands and 18/100"

### Python typing

- Typing is mandatory for function input arguments. The accepted input type should be as loose as possible (e.g. prefer `Sequence[T]` over `list[T]` if only indexing is required).
- Typing is discouraged for return types, unless it can not be inferred (e.g.: the return type is a supertype such as `list[SomeSuperType]`). If explicit, the hinted return type should be as the inted type should be as accurate as possible (e.g.: prefer `list[T]` over `Sequence[T]`).
- Python bindings are generated with `cargo run --features python-bindings --bin stub-gen`.

### Rust styling

You should run `cargo fmt` and `cargo clippy` before committing.

### Rust unit tests

Unlike the common convention, unit tests live in `src/unit_tests/`, one file per tested module. Each file is wired via a `#[path="..."]` directive by its tested module. Example from `src/solver/context.rs`:

```rust
#[cfg(test)]
#[path = "../unit_tests/test_context.rs"]
mod tests;
```

### Documentation

- Function documentation should be written in markdown format.
- Examples are appreciated if they help understand the function's behaviour.
- Function documentation can enumerate input arguments and explain their purpose, but not their type since they are already located in the function signature.
- When an exception can be raised in the function, the documentation should briefly explain under which conditions.

## Working principles

### Clause generation

The solver encodes a bounded multi-agent planning problem as a SAT formula. The encoding is built **incrementally**, one time step at a time, and is structured as three layers:

1. **`ClauseEngine`** — the low-level workhorse that knows how to produce the clauses for a _single_ time step `t`. It owns the static world geometry (via a `ConstraintContext`) and a `VarPool` that maps semantic variable keys (e.g. "agent 0 at position (2, 3) at time 4") to SAT literals. Its methods (`generate_movement_clauses`, `generate_laser_clauses`, `has_helped_by_time_clauses`, …) each return a `Vec<Clause>` for the requested step.

2. **`StepBuffer<T>`** — a self-filling, per-time-step cache. Each buffer wraps a `ClauseEngine` method pointer. When the solver asks for `gather_until(t)`, the buffer generates and caches every step it has not produced yet (in order), then returns the flattened contents of steps `0..=t`. Once a step is cached it is never regenerated, so solving at horizon 20 reuses all clauses from horizon 19.

3. **`ClauseGenerator`** — the public façade that orchestrates multiple `StepBuffer`s. It holds separate buffers for movements, lasers, cooperation tracking, etc. Its `generate(t, mode, collect_gems)` method composes the right subset of buffers depending on the `SolveMode` and appends the objective clause (every agent must be on an exit at time `t`). The generator is reusable across modes: shared domain buffers (e.g. movements) are filled once and read by every mode that needs them.

### Solver modes

To characterize a world, the solver must not only find _a_ solution but also prove that _no_ solution avoids a given property (cooperation, mutual help, etc.). This is done by combining two SAT queries:

1. **Existence proof** — solve in `Standard` mode to find a trajectory that exhibits property _P_ (e.g. the shortest plan involves cooperation). If the shortest plan does not exhibit _P_, the world trivially does not require it.

2. **Universality proof** — solve again with a `No*` mode (e.g. `NoCooperation`) that encodes _P_ as extra clauses and assumes _¬P_. If the SAT solver returns UNSAT, every trajectory of length ≤ `t_max` must exhibit _P_, proving the world requires it.

#### Feasibility shortcuts

Before dispatching on the mode, `ClauseGenerator::generate` checks whether _P_ can occur at all in the world layout. The check is deliberately cheap and horizon-independent: it only counts agents and potential helpers: the laser owners and the agents that may press a button pulsing a lift another agent may ride. For instance, a sequence of help edges needs two helpers, and fully coupled cooperation needs every agent to be one.

When those necessary conditions fail, _P_ is impossible, the `No*` restriction is tautologically satisfied, and the mode is normalized to `Standard`: the query returns the usual movement, laser, objective and gem clauses without filling the cooperation buffers or allocating cooperation variables. These are only *necessary* conditions — finer impossibility that depends on beam reachability or on `t_max` is left to the geometric pruning of the `ClauseEngine`, which never materializes a `Help` variable for an unrealizable help event.

#### Help events

`Help(h, b, t)` holds when, at `t`, `b` stands on a beam of `h` with no box upstream on that beam (so `h` must be the one blocking it), or when a lift lifted `b` during the step from `t - 1` to `t` and `h` is credited for an actuator that pulsed its group: `h` pressed the button, stands on the pressure plate, or is the last agent who toggled the lever (`LeverOwner`), which is on. Pushing a box is never cooperation: a beam blocked by a box is not help, whoever pushed the box. `NoCooperation` assumes every `Help` variable false, and the Python trajectory analyser (`lle.characterization.plan`) detects the same events.

### Boxes

`Box(b, x, t)` states that box $b$ is on cell $x$ at time $t$. Like agents, a box only gets variables on the cells it can actually reach (`relevant_box_positions`): its start cell, then any cell an agent could push it to. A box is pushed when an agent walks onto its cell, and moves one cell further in the same direction. The box clauses (`generate_box_clauses`) are part of the movement clauses:

- **Adjacency:** a box on $x$ at $t$ was on $x$ or a neighbour of $x$ at $t - 1$, never on a void. This is what destroys a box pushed into a void: it has no possible position afterwards.
- **Inertia:** a box on a non-void cell stays there unless an agent enters its cell (or walks onto it and is lifted away in the same step).
- **Push:** an agent stepping from $q - d$ onto the box's cell $q$ moves the box to $q + d$. This is impossible when $q + d$ cannot hold a box (wall, laser source, outside the grid).
- **No overlap:** two boxes, or an agent and a box, never share a cell, a void included.
- **No following:** a box cannot be pushed onto a cell an agent is leaving.

A box on a non-void beam tile blocks the beam, whatever its colour: `beam_activation` counts boxes as blockers next to the beam's owner.

### Lifts, buttons, levers and pressure plates

The encoding follows `World::step`, which resolves a step in three passes: every agent walks (pushing boxes), then the groups are pulsed (every agent taking `Trigger` presses the button or toggles the lever it stands on; a group is pulsed by a pressed button, a lever that is on, or a pressure plate with an allowed agent on it), then every pulsed lift moves the agent standing on it, if that agent may ride it, one layer up or down. These variables describe the step from $t$ to $t + 1$: `Button(b, t)` (button $b$ is pressed), `Toggle(l, t)` (lever $l$ is toggled), `Pulse(g, t)` (group $g$ is pulsed) and `Lifted(i, L, t)` (agent $i$ is lifted by lift $L$). `Lifted` variables only exist for the agents whose colour may ride $L$, and are created in `lift_moves`; every clause that reads them goes through `lifted_literals`.

- **Pulse:** `Pulse(g, t)` is the disjunction of the group's pressed buttons, of its levers that are on at $t + 1$ (a lever pulses in the very step it is switched on, and no longer in the step it is switched off), and of its plates occupied after the walk. A plate's occupancy is `Agent(i, p, t + 1)`, unless a lift may bring $i$ onto the plate: then an `OnPlate` variable excludes that lift. The lift clauses below only read `Pulse`.
- **Lever state:** `LeverOn(l, t + 1)` is `LeverOn(l, t)` xor `Toggle(l, t)`; levers start off, so no variable exists before the first possible toggle. A toggle needs an allowed agent on the lever, who stays there, like a press.

- **Press:** a pressed button or a toggled lever has an agent allowed to activate it on it, and that agent stays there.
- **Lift move:** a lifted agent stood on its lift or walked onto it at $t$ (both happen in one step), one button of the lift's group is pressed, and the agent is on the lift's destination at $t + 1$. Time-wise adjacency and the following conflict accept a lifted agent as an extra way to enter a cell.
- **Forced lift:** a pulsed lift never keeps an agent it may ride, unless another lift brought that agent there in this very step (a lift moves an agent once per step). This also forbids relying on the cases where the world cancels a lift (a box or another agent on the destination).
- **The rider is on the lift between the walk and the lift pass,** even when it walked onto it: no other agent may leave or walk onto the lift in that step (another lift of the group may still bring an agent onto it, swapping their riders), no box may be pushed onto it, and lasers are checked in that intermediate state too (`walk_pass_lasers`), where a lifted agent blocks the beams of its lift and not those of its destination.
- **Boxes:** the world never lifts onto a box, and a rider walking onto a box on its lift pushes it like `box_push` would.

Reachability (`ConstraintContext`) includes the lift moves, both in the exit distances and in the per-agent relevant positions, so that cells only reachable by lift are not pruned. `python/tests/solver/test_lifts.py` checks the solver on tricky worlds against the shortest plan length found by an exhaustive search over `World`.

### Levers and pressure plates

Two other actuators pulse the lifts of their group, next to buttons, with the same colour restriction. In `World::step`, the pulsed groups are gathered after the walk pass, in `pulsed_groups`:

- a **lever** (`H<g>`) is toggled by `Trigger`. While it is on, it pulses its group at every step, from the step it is switched on until the step it is switched off (excluded). Its on/off flag is the only actuator state that persists, so it is part of `WorldState` (`levers_on`);
- a **pressure plate** (`P<g>`) pulses its group at every step an authorized agent stands on it after the walk pass. It is not triggerable.
