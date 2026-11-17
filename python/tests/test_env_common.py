"""Tests of the physics-agnostic parts of the env layer: `lle.env.common`, the generic
`PotentialShaped` reward strategy, and the `UnsupportedFeatureError` exception."""

import pickle
from collections.abc import Collection, Sequence
from copy import deepcopy
from pathlib import Path

import lle
import numpy as np
import pytest
from lle import LLE, EventType, World, WorldEvent
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


class Constant(RewardStrategy):
    """A custom strategy, neither `SingleObjective` nor `MultiObjective`: always rewards 1."""

    def __init__(self, n_agents: int):
        super().__init__(n_agents, ["constant"])

    def compute_reward(self, events: Sequence[WorldEvent]):
        return np.array([1.0], dtype=np.float32)


class Pair(RewardStrategy):
    """A custom strategy with two objectives: always rewards [1, 2]."""

    def __init__(self, n_agents: int):
        super().__init__(n_agents, ["one", "two"])

    def compute_reward(self, events: Sequence[WorldEvent]):
        return np.array([1.0, 2.0], dtype=np.float32)


def test_potential_shaped_wraps_a_custom_single_objective_strategy():
    shaped, positions = make_shaped(Constant(2), CELLS)
    assert shaped.objectives == ["constant"]
    assert shaped.n_objectives == 1
    for (new_positions, events), s in zip(EPISODE, SHAPING):
        positions[:] = new_positions
        assert shaped.compute_reward(events) == pytest.approx([1.0 + s])


def test_potential_shaped_wraps_a_custom_multi_objective_strategy():
    shaped, positions = make_shaped(Pair(2), CELLS)
    assert shaped.objectives == ["one", "two", "PBRS"]
    assert shaped.n_objectives == 3
    for (new_positions, events), s in zip(EPISODE, SHAPING):
        positions[:] = new_positions
        assert shaped.compute_reward(events) == pytest.approx([1.0, 2.0, s])


def test_potential_shaped_wraps_a_potential_shaped_strategy():
    # The inner strategy shapes with group 0 only, the outer one with group 1 only, both reading
    # the same positions. The inner one has a single objective, so the outer one adds to it.
    positions = list(START)
    inner = PotentialShaped(SingleObjective(2), GAMMA, REWARD_VALUE, [CELLS[0]], lambda: positions)
    outer = PotentialShaped(inner, GAMMA, REWARD_VALUE, [CELLS[1]], lambda: positions)
    assert outer.objectives == ["reward"]
    # Per-group potentials (2 agents per group, 0.5 each) along EPISODE:
    # group 0: 1.0, 0.5, 0.5, 0.5, 0.0 and group 1: 1.0, 1.0, 1.0, 0.5, 0.5, both from 1.0.
    inner_shaping = [0.75 * 1.0 - 1.0, 0.75 * 1.0 - 0.5, 0.75 * 0.5 - 0.5, 0.75 * 0.5 - 0.5, 0.75 * 0.5 - 0.0]
    outer_shaping = [0.75 * 1.0 - 1.0, 0.75 * 1.0 - 1.0, 0.75 * 1.0 - 1.0, 0.75 * 1.0 - 0.5, 0.75 * 0.5 - 0.5]
    base = [0, 0, 0, 1, -1]
    for (new_positions, events), b, i, o in zip(EPISODE, base, inner_shaping, outer_shaping):
        positions[:] = new_positions
        assert outer.compute_reward(events) == pytest.approx([b + i + o])
    assert (outer.n_deads, outer.n_arrived) == (1, 0)


def test_potential_shaped_without_callback_must_be_subclassed():
    with pytest.raises(NotImplementedError):
        PotentialShaped(SingleObjective(1), GAMMA, REWARD_VALUE, CELLS)


def _move_agent_into_the_beam(env: LLE):
    """Put the single agent of `PBRS_MAP` on the first cell of the beam of `L0S`."""
    state = env.world.get_state()
    state.agents_positions = [(1, 1)]
    env.world.set_state(state)


PBRS_MAP = """
S0 L0S X
.  .   .
"""


def test_deepcopy_of_a_pbrs_env_shapes_with_its_own_world():
    env = lle.from_str(PBRS_MAP).pbrs().build()
    env.reset()
    copy = deepcopy(env)
    initial_potential = copy.reward_strategy.compute_potential()

    # Moving the agent of the original world into the beam changes the original's potential only.
    _move_agent_into_the_beam(env)
    assert env.reward_strategy.compute_potential() < initial_potential
    assert copy.reward_strategy.get_agents_positions() == copy.world.agents_positions == [(0, 0)]
    assert copy.reward_strategy.compute_potential() == initial_potential

    # The same move in the copy gives the copy the same shaping as the original.
    _move_agent_into_the_beam(copy)
    assert copy.reward_strategy.compute_potential() == env.reward_strategy.compute_potential()


def test_pbrs_env_pickles_and_shapes_with_its_own_world():
    env = lle.from_str(PBRS_MAP).pbrs().build()
    env.reset()
    restored = pickle.loads(pickle.dumps(env))
    initial_potential = restored.reward_strategy.compute_potential()

    _move_agent_into_the_beam(restored)
    assert env.world.agents_positions == [(0, 0)]
    assert restored.reward_strategy.compute_potential() < initial_potential
    assert env.reward_strategy.compute_potential() == initial_potential


def test_stub_declares_unsupported_feature_error_as_a_parsing_error():
    # `UnsupportedFeatureError` is declared by hand in `src/bindings/pyexceptions.rs` because
    # `pyo3_stub_gen::create_exception!` writes the base as `builtins.ParsingError`. This pins the
    # committed stub, so that an upgrade of `pyo3-stub-gen` that breaks the workaround is noticed
    # after running `cargo stub-gen`.
    stub = Path(lle.__file__).parent / "exceptions" / "__init__.pyi"
    assert "class UnsupportedFeatureError(ParsingError):" in stub.read_text()
