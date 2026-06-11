"""Examples of the CustomGenerator for composable world layouts.

CustomGenerator lets you control every placement decision explicitly rather
than picking a pre-canned strategy.  Run this file to see a quick demo:

    python examples/custom_generator.py
"""

import lle
from lle import CustomGenerator
from lle.generator.world_filter import WorldFilter

if __name__ == "__main__":
    # ------------------------------------------------------------------
    # Agents on the left edge, exits on the right edge
    # (lane-based, like `kind="constructive"`)
    # ------------------------------------------------------------------
    gen = CustomGenerator(
        width=8,
        height=8,
        n_agents=3,
        starts="edge",
        exits="opposite",
        n_walls=4,
        walls_style="shapes",
    )
    world = gen.generate(None, seed=1)
    assert world is not None
    print(world.world_string, "\n")

    # ------------------------------------------------------------------
    # Structural lasers: each beam crosses all agent lanes
    #  (requires starts="edge")
    # ------------------------------------------------------------------
    gen = CustomGenerator(
        width=8,
        height=8,
        n_agents=2,
        starts="edge",
        exits="opposite",
        n_lasers=2,
        laser_placement="cross-agent",
        n_walls=3,
    )
    world = gen.generate(None, seed=2)
    print("Cross-agent lasers (each beam spans all lanes)")
    assert world is not None
    print(world.world_string, "\n")

    # ------------------------------------------------------------------
    # Require cooperation: SAT-verify the generated world needs
    # laser blocking to be solvable
    # ------------------------------------------------------------------
    gen = CustomGenerator(
        width=8,
        height=8,
        n_agents=2,
        starts="edge",
        exits="opposite",
        n_lasers=1,
        laser_placement="cross-agent",
        filter=WorldFilter.cooperative(t_max=30),
    )
    world = gen.generate(seed=5, max_attempts=200)
    assert world is not None
    assert lle.is_cooperative(world)
    print("6. SAT-verified cooperative world")
    print(world.world_string, "\n")
