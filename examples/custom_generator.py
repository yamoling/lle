"""Composable world layouts with the generation builder.

`lle.generate(...)` returns a fluent builder that controls every placement
decision — start/exit geometry, lasers, walls, and the behavioural constraint —
through chained methods. Run this file to see a quick demo:

    python examples/custom_generator.py
"""

import lle

if __name__ == "__main__":
    # ------------------------------------------------------------------
    # Agents on one edge, exits on the opposite edge (one lane per agent),
    # with grouped wall shapes.
    # ------------------------------------------------------------------
    world = (
        lle.generate(width=8, height=8, n_agents=3)
        .lanes()
        .walls(4, style="shapes")
        .build(seed=1)
    )
    print(world.world_string, "\n")

    # ------------------------------------------------------------------
    # Structural lasers: each beam crosses every agent lane.
    # ------------------------------------------------------------------
    world = (
        lle.generate(width=8, height=8, n_agents=2)
        .lanes()
        .lasers(2, placement="cross-agent")
        .walls(3)
        .build(seed=2)
    )
    print("Cross-agent lasers (each beam spans all lanes)")
    print(world.world_string, "\n")

    # ------------------------------------------------------------------
    # Require cooperation: SAT-verify the generated world needs laser
    # blocking to be solvable.
    # ------------------------------------------------------------------
    world = (
        lle.generate(width=8, height=8, n_agents=2)
        .lanes()
        .lasers(1, placement="cross-agent")
        .cooperative(t_max=30)
        .build(max_attempts=200)
    )
    assert world is not None
    assert lle.is_cooperative(world)
    print("SAT-verified cooperative world")
    print(world.world_string, "\n")
