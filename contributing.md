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

Before dispatching on the mode, `ClauseGenerator::generate` checks whether _P_ can occur at all in the world layout. The check is deliberately cheap and horizon-independent: it only counts agents, laser sources and distinct laser colours (which are agent IDs, so they also count the agents able to act as helpers). For instance, a sequence of help edges needs two laser-owning helpers, and fully coupled cooperation needs every agent to own a laser.

When those necessary conditions fail, _P_ is impossible, the `No*` restriction is tautologically satisfied, and the mode is normalized to `Standard`: the query returns the usual movement, laser, objective and gem clauses without filling the cooperation buffers or allocating cooperation variables. These are only *necessary* conditions — finer impossibility that depends on beam reachability or on `t_max` is left to the geometric pruning of the `ClauseEngine`, which never materializes a `Help` variable for an unrealizable help event.
