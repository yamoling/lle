# Laser Learning Environment (LLE)
In LLE, agents start on start tiles, collect gems, and finish by reaching exit tiles. When an agent enters a laser of its own colour, it blocks the beam and lets the others pass; entering a laser of any other colour kills it and ends the episode. This single mechanic makes LLE a benchmark for **coordination-critical** cooperation.

📖 **Documentation:** [https://yamoling.github.io/lle/](https://yamoling.github.io/lle/)

![LLE](docs/lvl6-annotated.png)

## Highlights
- ⚡ **Fast** — game logic implemented in Rust, exposed to Python.
- 🤝 **Coordination-critical** — lasers can force agents to actively help each other to reach the exit.
- 🎚️ **Two levels of abstraction** — a high-level `LLE` MARL environment, or a low-level `World` for full control over maps, states, and steps.
- 🗺️ **Custom maps** — write a map as a one-line string or a richer TOML file, or use the 6 built-in levels.
- 🟰 **SAT Solver** — retrieve solutions to LLE worlds using a SAT-based solver.
- 🧪 **World analysis** — analyse the characteristics of a World: does it require cooperation? mutual cooperation?
- 🐣 **Procedural world generation** — generate worlds according to your requirements (cooperative, independent, mutually cooperative, ...)
- 🔍 **Rich observations** — layered, flattened, partial views, RGB images, and more, with optional reward shaping (PBRS) and multi-objective rewards.

## Installation
Install with `uv`, `pip`, `poetry`, …
```bash
pip install laser-learning-environment
```

## Quick start
LLE can be used at two levels of abstraction: as an `MARLEnv` for cooperative multi-agent reinforcement
learning, or as a `World` for fine-grained control.

### As a MARL environment
The `LLE` class wraps a `World` and implements the [`MARLEnv` interface](https://github.com/yamoling/multi-agent-rlenv) from the to add a reward function, observations, states, etc. Build one with `lle.level(...)`, `lle.from_str(...)`, or `lle.from_file(...)`, then chain builder methods before `build()`.

Here is an example on the following map: ![LLE](docs/3x1.png)

```python
import lle

env = lle.from_str("S0 G X").obs_type("layered").build()
obs, state = env.reset()
terminal = False
while not terminal:
    # env.render()                 # uncomment to render
    actions = env.sample_action()
    step = env.step(actions)
    # step.reward, step.obs, step.info, ...
    terminal = step.is_terminal # truncated or done
```

### As a `World` for fine-grained control
The `World` class exposes the state of the world and the events that happen when the agents move.

```python
from lle import World, Action, EventType

world = World("S0 G X")  # linear world: start S0, gem G, exit X
world.reset()
available = world.available_actions()[0]   # [Action.STAY, Action.EAST]

events = world.step([Action.EAST])
assert events[0].event_type == EventType.GEM_COLLECTED
events = world.step([Action.EAST])
assert events[0].event_type == EventType.AGENT_EXIT
```

You can save and restore the exact state of the world:
```python
import lle

world = lle.World.level(1)
state = world.get_state()
# ...
events = world.set_state(state)
```

Query the world through properties such as `world.start_pos`, `world.exit_pos`, `world.gems`,
`world.lasers`, and `world.agents`.

## Procedural generation, solving & analysis
The optional `generator` module provides procedural generation of proven solvable word capabilities. Call `lle.generate(...)`, chain with other methods to describe the characteristics of your world, and end with `build()` or `take(n=...)` to generate one or multiple worlds.

```bash
pip install laser-learning-environment[generator]
```

```python
import lle
from lle import World

# A solvable 5x5 world with 2 agents
world = lle.generate(width=5, height=5, n_agents=2).build(seed=0)

# Find the shortest joint plan (or None if unsolvable within t_max steps)
plan = lle.solve(world, 5)

# A world that *requires* cooperation, SAT-verified
coop = lle.generate(width=6, height=6, n_agents=2).lasers(2).cooperative().build()
assert lle.is_cooperative(coop, t_max=15)

# Prove what every short plan requires (e.g. level 6 is mutually cooperative)
assert lle.is_cooperative(World.level(6), t_max=25)
```

The builder controls every placement decision:
- **Layout:** `random()`, `lanes()`, `clustered()`, or fine-grained `starts(...)` / `exits(...)`.
- **Lasers & walls:** `lasers(n, placement=..., span=...)`, `walls(n, style=...)`.
- **Behaviour:** `solvable()` (default), `independent()`, `cooperative(...)`, `mutual(...)`.

```python
import lle 

world = lle.generate(width=5, height=5, n_agents=3).lanes().walls(4, style="shapes").build()
worlds = list(lle.generate(width=5, height=5, n_agents=2).clustered().lasers(2).mutual(t_max=10).take(5))
```

See the [`examples/`](examples) folder for runnable scripts and the
[documentation](https://yamoling.github.io/lle/) for the full API.

## Citing our work
The environment has been presented at [EWRL 2023](https://openreview.net/pdf?id=IPfdjr4rIs) and at
[BNAIC 2023](https://bnaic2023.tudelft.nl/static/media/BNAICBENELEARN_2023_paper_124.c9f5d29e757e5ee27c44.pdf)
where it received the best paper award.

```bibtex
@inproceedings{molinghen2023lle,
  title={Laser Learning Environment: A new environment for coordination-critical multi-agent tasks},
  author={Molinghen, Yannick and Avalos, Raphaël and Van Achter, Mark and Nowé, Ann and Lenaerts, Tom},
  year={2023},
  series={BeNeLux Artificial Intelligence Conference},
  booktitle={BNAIC 2023}
}
```

## Development
Clone the repo, install the Python dependencies, then compile with `maturin`. The example below uses
`uv`, but `conda`, `poetry`, or plain `pip` work too.
```bash
git clone https://github.com/yamoling/lle
uv venv                 # create a virtual environment
source .venv/bin/activate
uv sync                 # install python dependencies
maturin dev             # build and install lle in the venv
```

Re-generate the Python bindings in `python/lle` with:
```bash
cargo run --bin stub-gen
```

## Tests
Run the Rust and Python test suites with:
```bash
cargo test      # Rust unit + integration tests
maturin dev     # (re)build the extension
pytest          # Python tests
```
