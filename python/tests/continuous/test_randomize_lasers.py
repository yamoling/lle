"""Tests of `ContinuousWorld.set_laser_colour` and of the `randomize_lasers` builder option (round B, §4).

`ContinuousWorld.valid_laser_colours(laser_id)` is the single rule: a colour is valid for a source
when it is below `n_colours`, no possible start of an agent of another colour lies in the full beam
cells of that source, and its current beam would not kill an alive agent of another colour where it
stands. `set_laser_colour` accepts exactly these colours. With `.randomize_lasers()`, `CLLE.reset()`
draws, for each source in laser id order, a colour uniformly among its valid ones, with the Python
`random` module seeded by `reset(seed=...)` and `seed()`.
"""

import copy
import pickle

import lle.continuous as clle
import numpy as np
import pytest
from lle.continuous import ContinuousWorld
from lle.exceptions import InvalidWorldStateError

from .common import EAST, STILL, TWO_AGENTS

MAP = """
S0  .   .  .  X
L1E .   S1 .  X
.   .   .  .  .
S2  .   .  .  X
.   L0N .  .  .
"""
# Width 5, height 5, n_colours = 3. The beam of L1E on (1, 0) covers the start of agent 1 (colour
# 1): colour 1 is the only valid one for it. The beam of L0N on (4, 1) covers (3, 1) to (0, 1),
# where no agent starts: every colour is valid for it.
CONSTRAINED = (1, 0)
FREE = (4, 1)


def colour_by_cell(world: ContinuousWorld) -> dict[tuple[int, ...], int]:
    """The current colour of each laser source, by source cell `(i, j)`."""
    return {tuple(cell)[:2]: colour for cell, _, colour, _ in world.laser_sources}


def laser_id_at(world: ContinuousWorld, cell: tuple[int, int]) -> int:
    """The laser id of the source on `cell`."""
    return next(laser_id for c, _, _, laser_id in world.laser_sources if tuple(c)[:2] == cell)


def valid_colours(world: ContinuousWorld, source_index: int) -> set[int]:
    """The colours that the source at `source_index` (in laser order) may take, from the world."""
    return set(world.valid_laser_colours(world.laser_sources[source_index][3]))


def test_valid_colours_of_the_map():
    """A precondition of the other tests."""
    world = ContinuousWorld(MAP)
    by_cell = {tuple(cell)[:2]: index for index, (cell, *_) in enumerate(world.laser_sources)}
    assert valid_colours(world, by_cell[CONSTRAINED]) == {1}
    assert valid_colours(world, by_cell[FREE]) == {0, 1, 2}


def test_set_laser_colour_accepts_exactly_the_valid_colours():
    # On every built-in level and on MAP, after a reset and after a few random steps.
    worlds = [ContinuousWorld.level(n) for n in range(1, 7)] + [ContinuousWorld(MAP)]
    rng = np.random.default_rng(0)
    for world in worlds:
        for _ in range(3):
            for _, _, _, laser_id in world.laser_sources:
                valid = world.valid_laser_colours(laser_id)
                for colour in range(world.n_colours + 1):
                    copy_ = copy.deepcopy(world)
                    try:
                        copy_.set_laser_colour(laser_id, colour)
                        accepted = True
                    except InvalidWorldStateError:
                        accepted = False
                    assert accepted == (colour in valid), (laser_id, colour, valid)
            world.step(rng.uniform(-1, 1, (world.n_agents, 2)), 5)


def test_reset_draws_only_valid_colours():
    env = clle.from_str(MAP).randomize_lasers().build()
    for seed in range(20):
        env.reset(seed=seed)
        world = env.world
        for index, (_, _, colour, _) in enumerate(world.laser_sources):
            assert colour in valid_colours(world, index)
        assert colour_by_cell(world)[CONSTRAINED] == 1


def test_reset_draws_every_valid_colour_across_seeds():
    env = clle.from_str(MAP).randomize_lasers().build()
    drawn = set[int]()
    for seed in range(30):
        env.reset(seed=seed)
        drawn.add(colour_by_cell(env.world)[FREE])
    assert drawn == {0, 1, 2}


def test_the_same_seed_draws_the_same_colours():
    env = clle.from_str(MAP).randomize_lasers().build()
    draws = []
    for seed in range(5):
        env.reset(seed=seed)
        draws.append(colour_by_cell(env.world))
    for _ in range(3):
        env.reset()
    for seed in range(5):
        env.reset(seed=seed)
        assert colour_by_cell(env.world) == draws[seed]
        env.seed(seed)
        env.reset()
        assert colour_by_cell(env.world) == draws[seed]


def test_two_envs_with_the_same_seed_draw_the_same_colours():
    env1 = clle.from_str(MAP).randomize_lasers().build()
    env2 = clle.from_str(MAP).randomize_lasers().build()
    for seed in (3, 11):
        env1.reset(seed=seed)
        env2.reset(seed=seed)
        assert colour_by_cell(env1.world) == colour_by_cell(env2.world)


def test_randomization_never_changes_the_shapes():
    env = clle.from_str(MAP).obs_type("layered").state_type("layered").layered_resolution(1).randomize_lasers().build()
    n_colours = env.world.n_colours
    obs_shape, state_shape = env.observation_shape, env.state_shape
    assert obs_shape == (2 * 3 + 4, 5, 5)
    for seed in range(10):
        obs, state = env.reset(seed=seed)
        assert env.world.n_colours == n_colours
        assert env.observation_shape == obs_shape
        assert env.state_shape == state_shape
        assert obs.data.shape == (3, *obs_shape)
        assert state.data.shape == state_shape


def test_observations_follow_the_drawn_colours():
    # The source cells are -1 in the laser channel of their current colour (channel 3 + colour).
    env = clle.from_str(MAP).obs_type("layered").layered_resolution(1).randomize_lasers().build()
    for seed in range(10):
        obs, _ = env.reset(seed=seed)
        for (i, j), colour in colour_by_cell(env.world).items():
            assert obs.data[0, 3 + colour, i, j] == -1.0
            assert all(obs.data[0, 3 + c, i, j] == 0.0 for c in range(3) if c != colour)


def test_reset_restores_the_initial_colours_without_randomization():
    env = clle.from_str(MAP).build()
    env.reset()
    initial = colour_by_cell(env.world)
    env.world.set_laser_colour(laser_id_at(env.world, FREE), 2)
    assert colour_by_cell(env.world)[FREE] == 2
    env.reset()
    assert colour_by_cell(env.world) == initial


def test_set_laser_colour_recomputes_the_beam():
    # In TWO_AGENTS, the beam of L0N (laser 0, on (1, 2)) covers the cell (0, 2). Agent 1 (colour
    # 1) walking east into it dies, unless the source takes colour 1: then it blocks the beam.
    world = ContinuousWorld(TWO_AGENTS)
    killed = copy.deepcopy(world)
    killed.step([STILL, EAST], 10)
    assert killed.agents_alive == [True, False]

    world.set_laser_colour(0, 1)
    assert world.laser_sources[0][2] == 1
    layered = world.layered(1)  # channels: A0, A1, L0, L1, ...
    np.testing.assert_array_equal(layered[2], 0.0)
    assert layered[3, 1, 2] == -1.0 and layered[3, 0, 2] == 1.0
    outcome = world.step([STILL, EAST], 10)
    assert world.agents_alive == [True, True]
    assert outcome.n_ticks == 10


@pytest.mark.parametrize(
    "source, colour",
    [(FREE, 3), (FREE, -1), (CONSTRAINED, 0), (CONSTRAINED, 2), (None, 0), (-1, 0)],
    ids=[
        "colour-out-of-range",
        "negative-colour",
        "start-of-another-colour-0",
        "start-of-another-colour-2",
        "unknown-laser",
        "negative-laser-id",
    ],
)
def test_invalid_colour_changes_raise_and_change_nothing(source: tuple[int, int] | int | None, colour: int):
    world = ContinuousWorld(MAP)
    world.get_image()  # an already-rendered frame must not change either
    laser_id = 42 if source is None else source if isinstance(source, int) else laser_id_at(world, source)
    sources = world.laser_sources
    state = world.get_state()
    layered = world.layered(1)
    image = world.get_image()
    with pytest.raises(InvalidWorldStateError):
        world.set_laser_colour(laser_id, colour)
    assert world.laser_sources == sources
    assert world.get_state() == state
    np.testing.assert_array_equal(world.layered(1), layered)
    np.testing.assert_array_equal(world.get_image(), image)


@pytest.mark.parametrize("laser_id", [42, -1])
def test_valid_colours_of_an_unknown_laser_raise(laser_id: int):
    with pytest.raises(InvalidWorldStateError):
        ContinuousWorld(MAP).valid_laser_colours(laser_id)


UNSELECTED_START = """
world_string = \"\"\"
.   . . X
L0E . . .
.   . . X
\"\"\"
[[agents]]
start_positions = [{ i = 0, j = 0 }, { i = 1, j = 2 }]
[[agents]]
start_positions = [{ i = 2, j = 0 }]
"""
# Width 4, height 3, n_colours = 2. The beam of colour 0 runs over (1, 1) to (1, 3). Agent 0
# (colour 0) may start on (0, 0), outside the beam, or on (1, 2), inside it; agent 1 (colour 1)
# starts on (2, 0). Colour 1 would cross the possible start (1, 2) of agent 0, even when agent 0
# was drawn on (0, 0): colour 0 is the only valid one.


def test_recolouring_checks_the_starts_that_were_not_drawn():
    world = ContinuousWorld(UNSELECTED_START)
    # Draw starts until agent 0 starts outside the beam, on (0, 0).
    for seed in range(100):
        world.seed(seed)
        world.reset()
        if world.agents_positions[0] == (0.5, 0.5):
            break
    assert world.agents_positions[0] == (0.5, 0.5)
    assert world.valid_laser_colours(0) == [0]
    sources, state = world.laser_sources, world.get_state()
    layered, image = world.layered(2), world.get_image()
    with pytest.raises(InvalidWorldStateError):
        world.set_laser_colour(0, 1)
    assert world.laser_sources == sources
    assert world.get_state() == state
    np.testing.assert_array_equal(world.layered(2), layered)
    np.testing.assert_array_equal(world.get_image(), image)
    # Randomized resets never draw colour 1, whichever start agent 0 is drawn on.
    env = clle.from_str(UNSELECTED_START).randomize_lasers().build()
    starts = set()
    for seed in range(30):
        env.reset(seed=seed)
        starts.add(tuple(env.world.agents_cells[0])[:2])
        assert env.world.laser_sources[0][2] == 0
    assert starts == {(0, 0), (1, 2)}


def test_recolouring_never_puts_an_alive_agent_in_a_lethal_beam():
    # Mid-episode (through set_state), agent 1 (colour 1) stands at (2.5, 0.5) in the beam of
    # colour 0 of L0N (laser 0) in TWO_AGENTS: the beam would kill it at the next tick, so colour 1
    # is the only valid one, and setting it saves agent 1.
    world = ContinuousWorld(TWO_AGENTS)
    state = world.get_state()
    state.positions = [state.positions[0], (2.5, 0.5)]
    world.set_state(state)
    assert world.valid_laser_colours(0) == [1]
    world.set_laser_colour(0, 1)
    world.step([STILL, STILL], 1)
    assert world.agents_alive == [True, True]
    # Agent 1 now blocks its own beam: switching back to colour 0 would kill it where it stands.
    sources = world.laser_sources
    with pytest.raises(InvalidWorldStateError):
        world.set_laser_colour(0, 0)
    assert world.laser_sources == sources


def test_laser_colours_survive_copies_and_set_state():
    world = ContinuousWorld(MAP)
    world.set_laser_colour(laser_id_at(world, FREE), 2)
    world.set_state(world.get_state())
    assert colour_by_cell(world)[FREE] == 2
    for other in (copy.deepcopy(world), pickle.loads(pickle.dumps(world))):
        assert colour_by_cell(other)[FREE] == 2
        np.testing.assert_array_equal(other.layered(1), world.layered(1))
