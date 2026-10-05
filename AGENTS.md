# Agents instructions

@readme.md
@contributing.md

LLE (Laser Learning Environment) is a multi-agent reinforcement learning gridworld implemented as a Rust library with Python bindings via PyO3/maturin. Agents navigate a grid, collect gems, and reach exit tiles while avoiding or blocking laser beams.

## Python Binding Workflow

Each Rust type gets a `Py*` wrapper in `src/bindings/` deriving `#[pyclass]`. Custom PyO3 exceptions reside in `src/bindings/pyexceptions.rs`.

- **Critical Step:** After modifying Rust types exposed to Python, you MUST run `cargo stub-gen` to update the `.pyi` stubs.

## Map Formats

- **Plain-text (v1):** Space-separated tokens per row, newline-separated rows. Explained in `python/lle/__init__.py`.
  - Tokens: `S[id]` (Start), `G` (Gem), `X` (Exit), `.` (Floor), `@` (Wall), `V` (Void), `B` (Box), `L[id][direction]` (Laser source: N/E/S/W).
- **TOML (v2):** Richer format supporting random start positions. Automatically detected by the presence of a `[world]` header.
- Built-in levels 1–6 are statically embedded via `build.rs` and `src/core/levels.rs`.

## Constraints

- **Testing:** When running solver-related tests, always enforce a 60-second timeout to prevent infinite loops. The test structure should be `timeout 60 <test command>; [ $? -eq 124 ] && echo "=== Timeout reached ! ==="`. Examples:
  - `timeout 60 pytest; [ $? -eq 124 ] && echo "=== Timeout reached ! ==="`
  - `timeout 60 cargo test; [ $? -eq 124 ] && echo "=== Timeout reached ! ==="`
- For Rust tests, you should use the short output format with `cargo test -- --format terse`, unless you are debugging a specific test case.
