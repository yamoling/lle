"""Tests of the low-level `lle.continuous` bindings: `PhysicsParams`, `ContinuousWorld`,
`ContinuousWorldState` and `StepOutcome`.

The physics itself is covered by the Rust unit tests; these tests check that the bindings expose
it faithfully, and the error mapping of the contract (round A, §3).
"""

import copy
import pickle

import lle
import numpy as np
import pytest
from lle import EventType, WorldEvent
from lle.continuous import ContinuousWorld, ContinuousWorldState, PhysicsParams
from lle.exceptions import (
    InvalidActionError,
    InvalidLevelError,
    InvalidWorldStateError,
    ParsingError,
    UnsupportedFeatureError,
)
from lle.tiles import Direction

from .common import (
    CORRIDOR,
    EAST,
    GEM_THEN_EXIT,
    NORTH,
    RANDOM_STARTS,
    SAME_COLOUR,
    SOUTH,
    STILL,
    TWO_AGENTS,
    cells,
    tagged_events,
    xy,
)

# ---------------------------------------------------------------------------------------------
# PhysicsParams
# ---------------------------------------------------------------------------------------------


def test_params_defaults():
    params = PhysicsParams()
    assert params.dt == 0.05
    assert params.agent_radius == 0.3
    assert params.max_speed == 2.0
    assert params.action_model == "velocity"
    assert params.max_acceleration == 8.0
    assert params.damping == 0.9
    assert params.gem_radius == 0.25
    assert params.beam_width == 0.1


def test_params_keywords_and_attributes_are_read_write():
    params = PhysicsParams(dt=0.04, agent_radius=0.25, action_model="acceleration")
    assert (params.dt, params.agent_radius, params.action_model) == (0.04, 0.25, "acceleration")
    params.max_speed = 1.5
    params.max_acceleration = 5.0
    params.damping = 0.5
    params.gem_radius = 0.2
    params.beam_width = 0.05
    params.action_model = "velocity"
    assert params == PhysicsParams(
        dt=0.04,
        agent_radius=0.25,
        max_speed=1.5,
        action_model="velocity",
        max_acceleration=5.0,
        damping=0.5,
        gem_radius=0.2,
        beam_width=0.05,
    )
    assert params != PhysicsParams()


def test_params_repr_pickle_and_deepcopy():
    params = PhysicsParams(dt=0.04, action_model="acceleration")
    assert "PhysicsParams" in repr(params)
    assert pickle.loads(pickle.dumps(params)) == params
    assert copy.deepcopy(params) == params


def test_params_unknown_action_model_is_a_value_error():
    # Contract §3: the constructor and the setter raise, not only the world.
    with pytest.raises(ValueError):
        PhysicsParams(action_model="unicycle")
    params = PhysicsParams()
    with pytest.raises(ValueError):
        params.action_model = "unicycle"
    assert params.action_model == "velocity"


# ---------------------------------------------------------------------------------------------
# Construction
# ---------------------------------------------------------------------------------------------


def test_world_from_v1_string_exposes_the_map():
    world = ContinuousWorld(TWO_AGENTS)
    assert world.n_agents == 2
    assert (world.width, world.height) == (3, 2)
    assert world.n_colours == 2
    assert world.params == PhysicsParams()
    assert xy(world.agents_positions) == [(0.5, 0.5), (1.5, 0.5)]
    assert cells(world.agents_cells) == [(0, 0, 0), (0, 1, 0)]
    assert list(world.agents_colours) == [0, 1]
    assert list(world.agents_alive) == [True, True]
    assert list(world.agents_arrived) == [False, False]
    assert cells(world.gems_positions) == [(0, 2, 0)]
    assert list(world.gems_collected) == [False]
    assert cells(world.exits_positions) == [(1, 0, 0), (1, 1, 0)]
    assert [cells(starts) for starts in world.possible_starts] == [[(0, 0, 0)], [(0, 1, 0)]]
    assert [cells(beam) for beam in world.beam_cells] == [[(0, 2, 0)]]
    [(cell, direction, colour, laser_id)] = world.laser_sources
    assert tuple(cell) == (1, 2, 0)
    assert direction == Direction.NORTH
    assert (colour, laser_id) == (0, 0)


def test_world_from_toml_string_uses_its_colours():
    world = ContinuousWorld(SAME_COLOUR)
    assert list(world.agents_colours) == [0, 0]
    # max over agent and source colours, plus 1.
    assert world.n_colours == 1


def test_world_matches_the_discrete_world_on_level_6():
    """Same gems, exits, starts, colours and laser sources (ids, directions, beam cells)."""
    world = ContinuousWorld.level(6)
    discrete = lle.World.level(6)
    assert world.n_agents == discrete.n_agents == 4
    assert (world.width, world.height) == (discrete.width, discrete.height)
    assert world.n_colours == discrete.n_colours
    assert cells(world.gems_positions) == cells(g.pos for g in discrete.gems)
    assert cells(world.exits_positions) == cells(discrete.exit_pos)
    assert [cells(s) for s in world.possible_starts] == [[tuple(p)] for p in cells(discrete.start_pos)]
    sources = {tuple(cell): (direction, colour, laser_id) for cell, direction, colour, laser_id in world.laser_sources}
    assert len(sources) == len(discrete.laser_sources) == 3
    for source in discrete.laser_sources:
        direction, colour, laser_id = sources[tuple(source.pos)]
        assert (direction, colour, laser_id) == (source.direction, source.colour, source.laser_id)
        discrete_beam = [laser.pos for laser in discrete.lasers if laser.laser_id == source.laser_id]
        beam = cells(world.beam_cells[laser_id])
        assert sorted(beam) == sorted(cells(discrete_beam))
        # The beam cells are ordered from the source outwards.
        (i, j, k), (di, dj) = tuple(source.pos), direction.delta
        assert beam == [(i + n * di, j + n * dj, k) for n in range(1, len(beam) + 1)]


def test_beam_cells_are_ordered_from_the_source_outwards():
    world = ContinuousWorld("S0 . . L0W\nX . . .")
    assert cells(world.beam_cells[0]) == [(0, 2, 0), (0, 1, 0), (0, 0, 0)]


def test_level_and_from_file_accept_params(tmp_path):
    params = PhysicsParams(dt=0.04, agent_radius=0.25)
    assert ContinuousWorld.level(1, params).params == params
    assert ContinuousWorld.level(1).params == PhysicsParams()
    path = tmp_path / "same_colour.toml"
    path.write_text(SAME_COLOUR)
    world = ContinuousWorld.from_file(str(path), params)
    assert world.params == params
    assert list(world.agents_colours) == [0, 0]
    assert ContinuousWorld.from_file(str(path)).params == PhysicsParams()


def test_shapes_are_known_right_after_construction():
    # Level 6: 4 agents, 4 colours, 4 gems and 3 sources. Each agent block is 6 + 4 values.
    world = ContinuousWorld.level(6)
    assert world.state_size == 4 * 10 + 4 + 3
    assert world.observation_size == world.state_size
    assert world.state_vector().shape == (world.state_size,)
    assert world.observation(0).shape == (world.observation_size,)


# ---------------------------------------------------------------------------------------------
# Errors
# ---------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    "map_str, features",
    [
        ("S0 # X", ["boxes"]),
        ("S0 H0 P0 # X", ["boxes", "levers", "pressure_plates"]),
        ("S0 X\n;\n. .", ["layers"]),
    ],
)
def test_unsupported_features_raise_and_are_all_listed(map_str: str, features: list[str]):
    with pytest.raises(UnsupportedFeatureError) as error:
        ContinuousWorld(map_str)
    for feature in features:
        assert feature in str(error.value)


def test_unsupported_features_in_a_file_raise(tmp_path):
    path = tmp_path / "boxes"
    path.write_text("S0 # X")
    with pytest.raises(UnsupportedFeatureError):
        ContinuousWorld.from_file(str(path))


def test_invalid_map_is_a_parsing_error():
    with pytest.raises(ParsingError):
        ContinuousWorld("S0 X\nS1")


@pytest.mark.parametrize(
    "kwargs",
    [
        {"dt": 0.0},
        {"agent_radius": 0.5, "max_speed": 1.0},
        {"damping": 1.5},
        # Anti-tunnelling: max_speed * dt = 0.5 > agent_radius / 2.
        {"max_speed": 10.0},
    ],
)
def test_invalid_params_are_a_value_error(kwargs: dict[str, float]):
    # Whether `PhysicsParams` validates eagerly is not specified: the world must raise at the latest.
    with pytest.raises(ValueError) as error:
        ContinuousWorld(CORRIDOR, PhysicsParams(**kwargs))
    assert error.type is ValueError


@pytest.mark.parametrize("level", [0, 7, -1, 2**64, 2**70], ids=["zero", "seven", "negative", "u64-overflow", "huge"])
def test_unknown_level_is_an_invalid_level_error(level: int):
    with pytest.raises(InvalidLevelError):
        ContinuousWorld.level(level)


def test_missing_file_is_a_file_not_found_error():
    with pytest.raises(FileNotFoundError):
        ContinuousWorld.from_file("/this/file/does/not/exist")


@pytest.mark.parametrize(
    "actions",
    [
        [EAST],  # one action for two agents
        [EAST, EAST, EAST],
        np.zeros((2, 3)),  # wrong shape
        np.zeros(4),  # flat
    ],
    ids=["too-few", "too-many", "three-components", "flat"],
)
def test_wrong_number_or_shape_of_actions_is_a_value_error_without_side_effect(actions):
    world = ContinuousWorld(TWO_AGENTS)
    before = world.get_state()
    with pytest.raises(ValueError) as error:
        world.step(actions)
    assert error.type is ValueError
    assert world.get_state() == before


@pytest.mark.parametrize("bad", [np.nan, np.inf, -np.inf])
def test_non_finite_action_is_an_invalid_action_error_without_side_effect(bad: float):
    world = ContinuousWorld(TWO_AGENTS)
    before = world.get_state()
    with pytest.raises(InvalidActionError):
        world.step([STILL, (0.0, bad)], 5)
    assert world.get_state() == before


@pytest.mark.parametrize("n_ticks", [0, -1, -(2**70), 2**70], ids=["zero", "negative", "very-negative", "huge"])
def test_non_positive_or_huge_ticks_is_a_value_error(n_ticks: int):
    world = ContinuousWorld(TWO_AGENTS)
    before = world.get_state()
    with pytest.raises(ValueError) as error:
        world.step([EAST, STILL], n_ticks)
    assert error.type is ValueError
    assert world.get_state() == before


@pytest.mark.parametrize("agent", [2, 3, -1, 2**64, 2**70], ids=["n_agents", "beyond", "negative", "u64-overflow", "huge"])
def test_observation_of_an_unknown_agent_is_an_index_error(agent: int):
    world = ContinuousWorld(TWO_AGENTS)
    with pytest.raises(IndexError):
        world.observation(agent)


def test_observation_accepts_numpy_integers():
    world = ContinuousWorld(TWO_AGENTS)
    np.testing.assert_array_equal(world.observation(np.int64(1)), world.observation(1))


@pytest.mark.parametrize("seed", [-1, 2**64, 2**70], ids=["negative", "u64-overflow", "huge"])
def test_seed_out_of_range_is_a_value_error(seed: int):
    world = ContinuousWorld(RANDOM_STARTS)
    with pytest.raises(ValueError) as error:
        world.seed(seed)
    assert error.type is ValueError


def test_largest_seed_is_accepted():
    world = ContinuousWorld(RANDOM_STARTS)
    world.seed(2**64 - 1)
    world.reset()


@pytest.mark.parametrize("bad", [1.0, "1", None])
def test_non_integer_arguments_are_a_type_error(bad):
    world = ContinuousWorld(TWO_AGENTS)
    with pytest.raises(TypeError):
        world.observation(bad)
    with pytest.raises(TypeError):
        world.seed(bad)
    with pytest.raises(TypeError):
        ContinuousWorld.level(bad)


# ---------------------------------------------------------------------------------------------
# Stepping
# ---------------------------------------------------------------------------------------------


def test_diagonal_action_is_capped_at_full_speed():
    # The action is clipped to the unit disc: (1, 1) moves by 0.1 / sqrt(2) per axis per tick.
    world = ContinuousWorld("S0 . . .\n. . . .\n. . . X")
    world.step([(1.0, 1.0)], 3)
    expected = 0.5 + 3 * 0.1 / np.sqrt(2)
    np.testing.assert_allclose(xy(world.agents_positions), [(expected, expected)], rtol=1e-9)


def test_step_runs_one_tick_by_default():
    world = ContinuousWorld(CORRIDOR)
    outcome = world.step([EAST])
    assert outcome.n_ticks == 1
    assert list(outcome.events) == [] and list(outcome.ticks) == []
    np.testing.assert_allclose(xy(world.agents_positions), [(0.6, 0.5)], rtol=1e-6)
    assert world.get_state().tick == 1


def test_step_accepts_lists_and_float32_or_float64_arrays():
    for actions in ([EAST], np.array([EAST], dtype=np.float32), np.array([EAST], dtype=np.float64)):
        world = ContinuousWorld(CORRIDOR)
        assert world.step(actions, 3).n_ticks == 3
        np.testing.assert_allclose(xy(world.agents_positions), [(0.8, 0.5)], rtol=1e-6)


def test_step_events_are_lle_world_events_tagged_with_their_tick():
    world = ContinuousWorld(GEM_THEN_EXIT)
    outcome = world.step([(0.7, 0.0)], 50)
    assert all(type(e) is WorldEvent for e in outcome.events)
    assert tagged_events(outcome) == [
        (6, EventType.GEM_COLLECTED, 0),
        (21, EventType.AGENT_EXIT, 0),
    ]
    # Early stop: every agent has arrived after tick index 21.
    assert outcome.n_ticks == 22
    assert world.get_state().tick == 22
    assert list(world.gems_collected) == [True]
    assert list(world.agents_arrived) == [True]


def test_step_runs_no_tick_once_every_agent_has_arrived():
    world = ContinuousWorld(GEM_THEN_EXIT)
    world.step([EAST], 50)
    outcome = world.step([EAST], 5)
    assert outcome.n_ticks == 0
    assert list(outcome.events) == []


def test_step_stops_early_on_death():
    # Agent 0 overlaps the void (x > 1.7) at tick index 17 (x = 1.69 + 0.07 = 1.76).
    world = ContinuousWorld("S0 . V\nS1 . X\nX  . .")
    outcome = world.step([(0.7, 0.0), STILL], 50)
    assert tagged_events(outcome) == [(17, EventType.AGENT_DIED, 0)]
    assert outcome.n_ticks == 18
    assert list(world.agents_alive) == [False, True]


def test_step_does_not_stop_while_an_agent_has_not_arrived():
    world = ContinuousWorld("S0 X .\nS1 . X")
    outcome = world.step([(0.7, 0.0), STILL], 20)
    assert tagged_events(outcome) == [(7, EventType.AGENT_EXIT, 0)]
    assert outcome.n_ticks == 20
    assert list(world.agents_arrived) == [True, False]


def test_reset_restores_the_initial_state():
    world = ContinuousWorld(GEM_THEN_EXIT)
    initial = world.get_state()
    world.step([EAST], 50)
    world.reset()
    assert world.get_state() == initial
    assert list(world.gems_collected) == [False]
    assert world.get_state().tick == 0


# ---------------------------------------------------------------------------------------------
# State
# ---------------------------------------------------------------------------------------------


def test_state_constructor_and_read_write_attributes():
    state = ContinuousWorldState(
        positions=[(0.5, 0.5), (1.5, 0.5)],
        velocities=[(0.0, 0.0), (0.0, 0.0)],
        gems_collected=[False],
        agents_alive=[True, True],
        agents_arrived=[False, False],
        sources_enabled=[True],
        tick=0,
    )
    world = ContinuousWorld(TWO_AGENTS)
    assert world.get_state() == state
    state.positions = [(0.5, 0.5), (1.5, 0.75)]
    state.velocities = [(0.0, 0.0), (0.0, 1.0)]
    state.gems_collected = [True]
    state.agents_alive = [True, False]
    state.agents_arrived = [True, False]
    state.sources_enabled = [False]
    state.tick = 7
    assert xy(state.positions) == [(0.5, 0.5), (1.5, 0.75)]
    assert xy(state.velocities) == [(0.0, 0.0), (0.0, 1.0)]
    assert list(state.gems_collected) == [True]
    assert list(state.agents_alive) == [True, False]
    assert list(state.agents_arrived) == [True, False]
    assert list(state.sources_enabled) == [False]
    assert state.tick == 7
    assert repr(state) == (
        "ContinuousWorldState(positions=[(0.5, 0.5), (1.5, 0.75)], velocities=[(0.0, 0.0), (0.0, 1.0)], "
        "gems_collected=[True], agents_alive=[True, False], agents_arrived=[True, False], sources_enabled=[False], tick=7)"
    )


def test_state_equality_is_exact():
    world = ContinuousWorld(TWO_AGENTS)
    a = world.get_state()
    b = world.get_state()
    assert a == b
    b.positions = [(0.5 + 1e-15, 0.5), (1.5, 0.5)]
    assert a != b


def test_get_state_returns_a_copy():
    world = ContinuousWorld(TWO_AGENTS)
    state = world.get_state()
    state.tick = 99
    state.positions = [(0.5, 1.5), (1.5, 0.5)]
    assert world.get_state().tick == 0
    assert xy(world.agents_positions) == [(0.5, 0.5), (1.5, 0.5)]


def test_state_pickle_and_deepcopy():
    world = ContinuousWorld(TWO_AGENTS, PhysicsParams(action_model="acceleration"))
    world.step([(0.3, 0.7), (0.0, 1.0)], 3)
    state = world.get_state()
    assert any(v != (0.0, 0.0) for v in xy(state.velocities))
    assert pickle.loads(pickle.dumps(state)) == state
    copied = copy.deepcopy(state)
    assert copied == state
    copied.tick = 1000
    assert state.tick == 3


def test_set_state_updates_every_getter():
    # Agent 0 at (0.9, 1.4) is in the cell (1, 0); agent 1 at (1.6, 1.3) is in the cell (1, 1)
    # (0.707 apart, 0.4 from the source square). Hazards are only processed at the next tick.
    world = ContinuousWorld(TWO_AGENTS)
    state = world.get_state()
    state.positions = [(0.9, 1.4), (1.6, 1.3)]
    state.gems_collected = [True]
    state.agents_alive = [True, False]
    state.agents_arrived = [True, False]
    state.sources_enabled = [False]
    state.tick = 12
    world.set_state(state)
    assert world.get_state() == state
    assert xy(world.agents_positions) == [(0.9, 1.4), (1.6, 1.3)]
    assert cells(world.agents_cells) == [(1, 0, 0), (1, 1, 0)]
    assert list(world.gems_collected) == [True]
    assert list(world.agents_alive) == [True, False]
    assert list(world.agents_arrived) == [True, False]


@pytest.mark.parametrize(
    "field, value",
    [
        ("positions", [(0.5, 0.5)]),
        ("velocities", [(0.0, 0.0)] * 3),
        ("gems_collected", []),
        ("agents_alive", [True]),
        ("agents_arrived", [False, False, False]),
        ("sources_enabled", [True, True]),
        # Agent 0 inside the laser source square [2, 3] x [1, 2].
        ("positions", [(2.5, 1.5), (1.5, 0.5)]),
        # Agent 0 overlaps agent 1 (0.4 < 2r).
        ("positions", [(1.1, 0.5), (1.5, 0.5)]),
        # Agent 0 crosses the grid boundary.
        ("positions", [(0.1, 0.5), (1.5, 0.5)]),
        ("velocities", [(np.nan, 0.0), (0.0, 0.0)]),
    ],
)
def test_set_state_rejects_invalid_states_without_side_effect(field: str, value: list):
    world = ContinuousWorld(TWO_AGENTS)
    valid = world.get_state()
    state = world.get_state()
    setattr(state, field, value)
    with pytest.raises(InvalidWorldStateError):
        world.set_state(state)
    assert world.get_state() == valid


@pytest.mark.parametrize("action_model", ["velocity", "acceleration"])
def test_replay_after_set_state_is_bit_exact(action_model: str):
    """Restoring a snapshot, in the same world or in a fresh one, then replaying the same actions
    gives identical states and events."""
    params = PhysicsParams(action_model=action_model)
    world = ContinuousWorld.level(6, params)
    rng = np.random.default_rng(1)
    for _ in range(4):
        world.step(rng.uniform(-1, 1, (4, 2)), 5)
    snapshot = world.get_state()
    actions = [rng.uniform(-1, 1, (4, 2)) for _ in range(30)]

    def replay(w: ContinuousWorld):
        trace = []
        for a in actions:
            outcome = w.step(a, 5)
            trace.append((w.get_state(), tagged_events(outcome), outcome.n_ticks))
        return trace

    first = replay(world)
    world.set_state(snapshot)
    assert world.get_state() == snapshot
    assert replay(world) == first
    fresh = ContinuousWorld.level(6, params)
    fresh.set_state(snapshot)
    assert replay(fresh) == first


# ---------------------------------------------------------------------------------------------
# Seeding and copies
# ---------------------------------------------------------------------------------------------


def test_seed_gives_reproducible_random_starts():
    world = ContinuousWorld(RANDOM_STARTS)
    centres = {(j + 0.5, i + 0.5) for i in range(2) for j in range(3)}
    assert [set(cells(s)) for s in world.possible_starts] == [{(i, j, 0) for i in range(2) for j in range(3)}] * 2
    seen = set()
    for seed in range(20):
        world.seed(seed)
        world.reset()
        positions = xy(world.agents_positions)
        world.seed(seed)
        world.reset()
        assert xy(world.agents_positions) == positions
        assert positions[0] != positions[1]
        assert set(positions) <= centres
        seen.add(tuple(positions))
    assert len(seen) > 1, "the random starts never depend on the seed"


@pytest.mark.parametrize("duplicate", [copy.deepcopy, lambda world: pickle.loads(pickle.dumps(world))], ids=["deepcopy", "pickle"])
def test_copies_are_independent_and_copy_the_rng(duplicate):
    world = ContinuousWorld(RANDOM_STARTS)
    world.seed(3)
    world.reset()
    clone = duplicate(world)
    assert clone.get_state() == world.get_state()
    # Same RNG state: the next random starts are the same.
    for _ in range(5):
        world.reset()
        clone.reset()
        assert clone.get_state() == world.get_state()
    # Independent: reseeding, resetting or stepping the clone leaves the original untouched,
    # including its RNG.
    reference = duplicate(world)
    before = world.get_state()
    clone.seed(1000)
    clone.reset()
    clone.step([SOUTH, STILL], 3)
    assert world.get_state() == before
    starts = set()
    for _ in range(5):
        world.reset()
        reference.reset()
        assert world.get_state() == reference.get_state()
        starts.add(tuple(xy(world.agents_positions)))
    assert len(starts) > 1, "the starts are not random: the test would not detect a lost RNG"
    # Reseeding the copy and the original alike gives the same starts again.
    world.seed(7)
    clone.seed(7)
    for _ in range(5):
        world.reset()
        clone.reset()
        assert clone.get_state() == world.get_state()


def test_pickle_preserves_the_map_params_and_state():
    params = PhysicsParams(action_model="acceleration", damping=0.8)
    world = ContinuousWorld(SAME_COLOUR, params)
    world.step([EAST, NORTH], 4)
    restored = pickle.loads(pickle.dumps(world))
    assert restored.params == params
    assert restored.get_state() == world.get_state()
    assert list(restored.agents_colours) == [0, 0]
    assert (restored.width, restored.height, restored.n_agents) == (world.width, world.height, world.n_agents)
    assert [cells(b) for b in restored.beam_cells] == [cells(b) for b in world.beam_cells]
    # The restored world steps exactly like the original one.
    for _ in range(5):
        a = world.step([(0.2, 0.9), (-0.5, -0.5)], 3)
        b = restored.step([(0.2, 0.9), (-0.5, -0.5)], 3)
        assert tagged_events(a) == tagged_events(b)
        assert restored.get_state() == world.get_state()
