# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository. Check out the [readme.md](readme.md) file as well.

## What this project is

LLE (Laser Learning Environment) is a multi-agent reinforcement learning gridworld implemented as a Rust library with Python bindings via PyO3/maturin. Agents navigate a grid, collect gems, and reach exit tiles while avoiding or blocking laser beams.

## Commands

### Build

```bash
maturin dev          # compile Rust and install into the venv
maturin dev --release  # optimised build
```

Python dependencies and the venv are managed with `uv`:

```bash
uv sync              # install Python deps
source .venv/bin/activate
```

### Tests

```bash
cargo test                      # Rust unit + integration tests
uv run pytest                          # Python tests (requires maturin dev first)
uv run pytest python/tests/test_world.py  # single Python test file
cargo test world_integration    # single Rust integration test file by name
```

When testing components of the solver, always use a timeout of 60 seconds in case there is an infinite loop.

### Benchmarking
When benchmarking a component, create a new folder under `benchmarks/<your-benchmark-name>/`. Every file that you need to perform the benchmark should be located there. Th create a benchmark:

1. Measure what you are asked to benchmark
2. Log your data to persisted files of an appropriate format (e.g. CSV or JSON)
3. If applicable, create some plots via some python scripts and matplotlib
4. Write a markdown report with:
    - a short introduction (what you are bechmarking)
    - a short methodology (how many repetitions, what you are measuring exactly and how)
    - the results in the form of a table and with plots (if applicable)
    - a brief conclusion

### Type checking and stubs

```bash
basedpyright                    # Python type checker
cargo run --bin stub-gen        # regenerate python/lle/*.pyi stubs
```

## Architecture

The Python `World` class (`python/lle/world/`) is a thin wrapper over the Rust `PyWorld`. The `LLE` class adds observation construction, reward shaping, and the `marlenv` interface on top.

### Map format

**Plain-text (v1):** space-separated tokens per row, newline-separated rows.
- `S0`, `S1`, … — agent start positions
- `G` — gem, `X` — exit, `.` — floor, `@` — wall, `V` — void
- `L0N`, `L1E`, … — laser source (agent id + direction N/E/S/W)

**TOML (v2):** richer format supporting random start positions and named fields; detected automatically by `[world]` header presence.

Built-in levels 1–6 are embedded via `build.rs` and `src/core/levels.rs`.

### Python binding conventions

Each Rust type gets a `Py*` wrapper in `src/bindings/` that derives `#[pyclass]`. Exceptions are custom PyO3 exception types in `src/bindings/pyexceptions.rs`. After changing Rust types exposed to Python, run `cargo run --bin stub-gen` to update the `.pyi` stubs.

# Others
- Unless explicitly requested, you may not create a new commit.
