## Plan: `WorldLayoutBuilder`

### What it is

A standalone builder-pattern class (`WorldLayoutBuilder`) that exposes explicit control over every placement decision, producing a `CustomGenerator` internally. Unlike the existing three generators (which encode fixed strategies), the builder lets users compose placement modes freely.

---

### New parameters and their values

| Category | Parameter | Values |
|---|---|---|
| **Agent starts** | `placement` | `"random"`, `"edge"`, `"clustered"` |
| **Exits** | `placement` | `"random"`, `"edge"`, `"cluster"`, `"opposite"` |
| **Walls** | `n` | int or `"auto"` (~10% of grid) |
| | `style` | `"individual"` (single cells), `"shapes"` (bars/L/2×2) |
| **Lasers** | `n` | int |
| | `placement` | `"free"`, `"cross-agent"` (crosses all agent lanes), `"cross-cluster"` (between start/end clusters) |
| |`span` | `"any"` (no constraint), `"across"` (the laser must span across the whole width or height), `n` (an int value with the minimal laser span) |

---

### Invalid placements
- A laser source can never directly face a wall (because there is no beam and the laser is useless)
- A laser must always span more than 1 tile, otherwise it can never be blocked for another agent.

### API sketch

```python
filter = WorldFilter.cooperative(t_ax, t_min=t_min)
generator = CustomGenerator(
    width=8, 
    height=8, 
    n_agents=2, 
    filter=filter,
    starts="edge",
    exits="opposite",          # auto-mirrors to right
    n_lasers=1
    laser_placement="free",
    laser_span=6, # Span at least 6 tiles
    n_walls=10,
    walls_style="shapes",
)
```

---

### Files to create / modify

| File | Action | Purpose |
|---|---|---|
| `generator/_shapes.py` | **Create** | Extract `_WALL_SHAPES` + `place_wall_shapes()` from `Level6StyleGenerator` into a standalone module |
| `generator/custom.py` | **Create** | `CustomGenerator(Generator)` — dispatches to private placement methods per config |
| `generator/level6_style.py` | **Modify** | Delegate `_place_wall_shapes` to `_shapes.py` |

---

### Implementation steps

**Step 1 — `_shapes.py`**: Move `_WALL_SHAPES` constant and `_place_wall_shapes` logic out of `Level6StyleGenerator` into a pure module-level function `place_wall_shapes(free_cells, budget, rng)`. Update `level6_style.py` to delegate. This avoids `CustomGenerator` inheriting from `Level6StyleGenerator`.

**Step 2 — `_geometry.py` refactor**: Move `_geometry_ok` from `RandomGenerator` into `_geometry.py` as a standalone function (currently it's a method that only uses `beam_tiles` and `points_out_immediately`, which already live there). `RandomGenerator` then calls the module function. `CustomGenerator` reuses it.

**Step 3 — `custom.py`**: `CustomGenerator(Generator)` stores four config dataclasses (`_AgentConfig`, `_ExitConfig`, `_LaserConfig`, `_WallConfig`). Its `_make_candidate_layout()` runs:
1. `_place_agents()` → dispatches on mode
2. `_place_exits(reserved)` → dispatches on mode; `"opposite"` is resolved from agent config
3. `_place_lasers(reserved)` → dispatches on placement; raises `_LayoutRetry` if no valid placement
4. `_place_walls(reserved)` → dispatches on style
5. Geometry validation → raises `_LayoutRetry` on failure

**Step 4 — exports**: Export relevant pieces in `__init__.py`

**Step 5 — tests**: Cover each placement mode (agents, exits, walls, lasers), the `"opposite"` exit resolution, filter integration, and validation errors.

---

### Key design decisions

- **`"structural"` / `"corridor"` laser modes encode the cooperation idioms** — users don't have to know the geometry tricks; picking `structural` + cooperative filter naturally produces the right kind of world.
