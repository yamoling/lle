"""Tests of the physics-agnostic parts of the env layer: `lle.env.common`, the generic
`PotentialShaped` reward strategy, and the `UnsupportedFeatureError` exception."""

from collections.abc import Collection, Sequence

import numpy as np
import pytest
from lle import EventType, World, WorldEvent
from lle.env import DeathStrategy, PotentialShaped
from lle.env.reward_strategy import MultiObjective, PotentialShapedLLE, RewardStrategy, SingleObjective
from lle.exceptions import ParsingError, UnsupportedFeatureError

GAMMA = 0.75
REWARD_VALUE = 0.5
START = [(0, 0, 0), (2, 2, 0)]
CELLS = [{(0, 1, 0), (0, 2, 0)}, {(1, 0, 0)}]
"""Two groups of rewarded cells, as for two laser beams."""

# Hand-made episode for 2 agents and the 2 groups of `CELLS`, as (positions, events).
#
# The potential is REWARD_VALUE times the number of (agent, group) pairs where the agent has not
# yet stood on a cell of the group, and the shaping term is GAMMA * previous - current potential.
# Initially, nobody is on a rewarded cell: potential = 0.5 * 4 = 2.0.
EPISODE = [
    # Nobody moves: potential 2.0, shaping 0.75 * 2.0 - 2.0 = -0.5.
    ([(0, 0, 0), (2, 2, 0)], []),
    # Agent 0 reaches group 0: potential 1.5, shaping 0.75 * 2.0 - 1.5 = 0.
    ([(0, 1, 0), (2, 2, 0)], []),
    # Agent 0 leaves the group, which stays reached: potential 1.5, shaping 0.75 * 1.5 - 1.5 = -0.375.
    ([(0, 0, 0), (2, 2, 0)], []),
    # Agent 0 on another cell of group 0 (already reached), agent 1 reaches group 1 and collects a
    # gem: potential 1.0, shaping 0.75 * 1.5 - 1.0 = 0.125.
    ([(0, 2, 0), (1, 0, 0)], [WorldEvent(EventType.GEM_COLLECTED, 1)]),
    # Agent 1 reaches group 0 and agent 0 dies: potential 0.5, shaping 0.75 * 1.0 - 0.5 = 0.25.
    ([(0, 2, 0), (0, 1, 0)], [WorldEvent(EventType.AGENT_DIED, 0)]),
]
SHAPING = [-0.5, 0.0, -0.375, 0.125, 0.25]


def make_shaped(strategy: RewardStrategy, cells: Sequence[Collection[tuple[int, int, int]]]):
    """
    A `PotentialShaped` strategy whose agents are at the positions held by the returned list,
    which the test updates in place.

    @ai-generated
    """
    positions = list(START)
    shaped = PotentialShaped(
        strategy=strategy,
        gamma=GAMMA,
        reward_value=REWARD_VALUE,
        pos_to_reward=cells,
        agents_positions=lambda: positions,
    )
    return shaped, positions


def test_death_strategy_is_the_same_object_from_every_import_path():
    from lle.env.common import DeathStrategy as from_common
    from lle.env.env import DeathStrategy as from_env_module

    assert from_common is from_env_module is DeathStrategy


def test_unsupported_feature_error_is_a_parsing_error():
    assert issubclass(UnsupportedFeatureError, ParsingError)
    assert issubclass(UnsupportedFeatureError, ValueError)


def test_potential_shaped_single_objective_adds_the_shaping_to_the_reward():
    shaped, positions = make_shaped(SingleObjective(2), CELLS)
    assert shaped.objectives == ["reward"]
    assert shaped.n_objectives == 1
    # The base rewards are: gem +1 at step 4, death -1 at step 5.
    expected = [s + base for s, base in zip(SHAPING, [0, 0, 0, 1, -1])]
    for (new_positions, events), reward in zip(EPISODE, expected):
        positions[:] = new_positions
        assert shaped.compute_reward(events) == pytest.approx([reward])
    assert shaped.n_deads == 1
    assert shaped.n_arrived == 0


def test_potential_shaped_multi_objective_appends_a_pbrs_objective():
    # Any collection of cells works, not only sets.
    shaped, positions = make_shaped(MultiObjective(2), [list(cells) for cells in CELLS])
    assert shaped.objectives == ["gem", "exit", "death", "done", "PBRS"]
    assert shaped.n_objectives == 5
    # [gem, exit, death, done]: a death zeroes the other base objectives, never the shaping.
    base = [[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [1, 0, 0, 0], [0, 0, -1, 0]]
    for (new_positions, events), b, s in zip(EPISODE, base, SHAPING):
        positions[:] = new_positions
        assert shaped.compute_reward(events) == pytest.approx(b + [s])


def test_potential_shaped_reset_forgets_reached_cells_and_reads_the_current_positions():
    shaped, positions = make_shaped(SingleObjective(2), CELLS)
    for new_positions, events in EPISODE:
        positions[:] = new_positions
        shaped.compute_reward(events)

    # Back on the starts: potential 2.0 again, so staying gives 0.75 * 2.0 - 2.0 = -0.5.
    positions[:] = START
    shaped.reset()
    assert (shaped.n_deads, shaped.n_arrived) == (0, 0)
    assert shaped.compute_potential() == pytest.approx(2.0)
    assert shaped.compute_reward([]) == pytest.approx([-0.5])

    # Reset with agent 1 on group 1: that pair counts as reached from the start, so the potential
    # is 1.5 and staying gives 0.75 * 1.5 - 1.5 = -0.375.
    positions[:] = [(0, 0, 0), (1, 0, 0)]
    shaped.reset()
    assert shaped.compute_reward([]) == pytest.approx([-0.375])


@pytest.mark.parametrize("strategy_class", [SingleObjective, MultiObjective])
def test_potential_shaped_lle_matches_potential_shaped_on_level_6(strategy_class: type[RewardStrategy]):
    world = World.level(6)
    world.reset()
    sources = world.laser_sources
    cells = [{laser.pos for laser in world.lasers if laser.laser_id == source.laser_id} for source in sources]
    reference = PotentialShapedLLE(strategy_class(world.n_agents), world, 0.99, 0.5, sources)
    shaped = PotentialShaped(
        strategy=strategy_class(world.n_agents),
        gamma=0.99,
        reward_value=0.5,
        pos_to_reward=cells,
        agents_positions=lambda: world.agents_positions,
    )
    assert shaped.objectives == reference.objectives

    rng = np.random.default_rng(0)
    potentials = set()
    for _ in range(500):
        actions = [available[rng.integers(len(available))] for available in world.available_actions()]
        events = world.step(actions)
        assert np.array_equal(shaped.compute_reward(events), reference.compute_reward(events))
        assert (shaped.n_deads, shaped.n_arrived) == (reference.n_deads, reference.n_arrived)
        potentials.add(shaped.compute_potential())
        died = any(event.event_type == EventType.AGENT_DIED for event in events)
        if died or all(agent.has_arrived for agent in world.agents):
            world.reset()
            reference.reset()
            shaped.reset()
    # The random walk does cross some lasers, so the comparison covers changes of potential.
    assert len(potentials) > 1
