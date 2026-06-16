# Agents instructions

Check out the [readme.md](readme.md) to get started and [contributing.md](contributing.md) to follow code guidelines. As an agent, you are not allowed to create commits on your own initiative, unless explicitly requested.

## Project description
LLE (Laser Learning Environment) is a multi-agent reinforcement learning gridworld implemented as a Rust library with Python bindings via PyO3/maturin. Agents navigate a grid, collect gems, and reach exit tiles while avoiding or blocking laser beams.

## Tests
When running tests (especially those related to the solver), always use a timeout of 60 seconds to avoid infinite loops.

## Benchmarking
To benchmark a component, create a new folder under `benchmarks/<your-benchmark-name>/`. Every file that you need to perform the benchmark should be located there. To create a benchmark:

1. Measure what you are asked to benchmark (durations, number of clauses, ...)
2. Persist your measurements to files in an appropriate format (typically JSON or CSV)
3. If applicable, create some plots via some python scripts and matplotlib
4. Write a markdown report with:
    - a short introduction (what you are bechmarking)
    - a short methodology (how many repetitions, what you are measuring exactly and how)
    - the results in the form of a table and with plots (if applicable)
    - a brief conclusion

## Architecture
The Python `World` class (`python/lle/world/`) is a thin wrapper over the Rust `PyWorld`. The `LLE` class adds observation construction, reward shaping, and the `marlenv` interface on top.

### Map format
**Plain-text (v1):** space-separated tokens per row, newline-separated rows.
- `S0`, `S1`, … — agent start positions
- `G` — gem, `X` — exit, `.` — floor, `@` — wall, `V` — void
- `L0N`, `L1E`, … — laser source (agent id + direction N/E/S/W)

The complete map format is explained in [python/lle/__init__.py](python/lle/__init__.py).

**TOML (v2):** richer format supporting random start positions and named fields; detected automatically by `[world]` header presence.

Built-in levels 1–6 are embedded via `build.rs` and `src/core/levels.rs`.

### Python binding conventions

Each Rust type gets a `Py*` wrapper in `src/bindings/` that derives `#[pyclass]`. Exceptions are custom PyO3 exception types in `src/bindings/pyexceptions.rs`. After changing Rust types exposed to Python, run `cargo run --bin stub-gen` to update the `.pyi` stubs.
