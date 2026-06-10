# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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

### Rust core (`src/`)

```
src/
  lib.rs              – crate root; re-exports public API
  core/               – all game logic (no Python coupling)
    world/world.rs    – World struct: reset/step/set_state/get_state
    tiles/tile.rs     – Tile enum (Floor, Wall, Laser, LaserSource, Gem, Exit, Void)
    parsing/          – two parsers: parser_v1 (plain-text), toml/ (TOML v2)
    errors.rs         – RuntimeWorldError, ParseError
    levels.rs         – built-in levels embedded at compile time
  bindings/           – PyO3 wrappers; one Py* struct per Rust type
    world/pyworld.rs  – PyWorld exposes World to Python
    tiles/            – PyLaser, PyLaserSource, PyGem, PyDirection
  action.rs           – Action enum (North/South/East/West/Stay)
  agent.rs            – Agent + AgentId
  position.rs         – Position {i, j} with Add<Action>
  rendering/          – image rendering via the `image` crate
  stub_gen.rs         – binary that writes .pyi files via pyo3-stub-gen
```

**Map parsing flow:** `parse()` tries TOML first, falls back to v1 plain-text. Both produce a `WorldConfig`, which calls `WorldConfig::to_world()` to construct a `World`.

**Step flow:** `World::step()` validates actions, computes new positions, resolves vertex conflicts (two agents targeting the same cell both stay), then calls `move_agents()` → `tile.leave()` / `tile.pre_enter()` / `tile.enter()`. Death from a laser triggers a second pass of `move_agents()` until no further deaths occur.

**Tile wrapping:** A laser beam cell is represented as `Tile::Laser(Laser)` where the `Laser` can wrap another tile (e.g. `Laser` over a `Gem`). Always unwrap when querying inner state.

### Python package (`python/lle/`)

```
python/lle/
  __init__.py         – public API; re-exports World, Action, LLE, generate, solve, …
  env/env.py          – LLE class: marlenv MARLEnv wrapper around World
  env/builder.py      – builder DSL: lle.level(6).obs_type("layered").build()
  observations.py     – ObservationType enum; numpy array construction
  solver/             – SAT-based solver (pysat/Minisat22)
    _constraints/     – clause generators: movements, lasers, objective, init
    incremental_solver.py – incremental clause reuse
  generator/          – procedural world generation
    random.py         – random layout + SAT solvability check
    constructive.py   – lane-based layout
    level6_style.py   – level-6-inspired clustered layout
```

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
