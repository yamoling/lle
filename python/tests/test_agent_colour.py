"""TDD tests for `.agents/plans/agent-colour-id.md`: agent ID and agent colour become independent.

These tests fail today. Observation-level behaviour is pinned in `test_observations.py` instead;
this file covers the world, the level format and the bindings.
"""

import pytest
from lle import Action, World

# Two agents of colour 1: repeated `S<c>` tokens declare several agents of one colour (§3.4).
SHARED_COLOUR_V1 = """
S1  .  .  X
L1E .  .  .
.   S1 .  X
"""

# The same world through the TOML `colour` key, which also works for non-adjacent numbering.
SHARED_COLOUR_TOML = """
world_string = \"\"\"
S0 .  .  X
L0E .  .  .
.   S1 .  X
\"\"\"
[[agents]]
colour = 0
[[agents]]
colour = 0
"""


def test_agent_colour_defaults_to_id():
    world = World("S0 . S1 X X")
    world.reset()
    for num, agent in enumerate(world.agents):
        assert agent.colour == num


def test_repeated_token_declares_agents_of_one_colour():
    world = World("S0 S0 X X")
    world.reset()
    assert world.n_agents == 2
    assert [agent.colour for agent in world.agents] == [0, 0]
    assert world.n_colours == 1


def test_agent_ids_are_colour_major():
    world = World("S1 S1 S0 X X X")
    world.reset()
    assert [agent.colour for agent in world.agents] == [0, 1, 1]
    assert world.agents_positions == [(0, 2), (0, 0), (0, 1)]


def test_sparse_colours():
    world = World("S0 S2 X X")
    world.reset()
    assert [agent.colour for agent in world.agents] == [0, 2]
    assert world.n_colours == 3


def test_toml_colour_key():
    world = World(SHARED_COLOUR_TOML)
    world.reset()
    assert [agent.colour for agent in world.agents] == [0, 0]


def test_same_colour_agent_survives_beam():
    world = World(SHARED_COLOUR_V1)
    world.reset()
    world.step([Action.STAY, Action.NORTH])
    assert all(agent.is_alive for agent in world.agents)


def test_different_colour_agent_still_dies_on_beam():
    world = World("S0 .  .  X\nL0E .  .  .\n.   S1 .  X")
    world.reset()
    world.step([Action.STAY, Action.NORTH])
    assert not world.agents[1].is_alive


def test_laser_source_exposes_colour():
    world = World(SHARED_COLOUR_V1)
    source = world.laser_sources[0]
    assert source.colour == 1
    # `agent_id` is kept for one release as a deprecated alias.
    assert source.agent_id == source.colour


def test_set_colour_beyond_n_agents_is_allowed():
    # A colour need not correspond to an agent, so the old `>= n_agents` bound check is gone.
    world = World("S0 L0E X")
    world.laser_sources[0].set_colour(3)
    assert world.laser_sources[0].colour == 3
    assert world.n_colours == 4


def test_randomize_lasers_samples_declared_colours():
    from lle import LLE

    env = LLE.from_str("S0 S2 X X\nL0E . . .").randomize_lasers().build()
    for _ in range(20):
        env.reset()
        for source in env.world.laser_sources:
            assert source.colour < env.world.n_colours


def test_solver_rejects_shared_colours():
    from lle.solver import Solver

    with pytest.raises(ValueError):
        Solver(World("S0 S0 X X"))
