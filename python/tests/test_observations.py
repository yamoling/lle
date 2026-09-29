from typing import get_args

import numpy as np
import pytest
from lle import LLE, Action, ObservationType, World
from lle.observations import Layered, ObservationTypeLiteral, PartialGenerator, PerspectiveLayered


def test_typing_observation_type_literal():
    for s in get_args(ObservationTypeLiteral):
        ok = False
        for o in ObservationType:
            if s == o:
                ok = True
                break
        assert ok, f"{s} is not a valid ObservationType"
    for o in ObservationType:
        assert o in get_args(ObservationTypeLiteral)


def test_observation_gem_collected():
    world = World(
        """
S0 X . .
.  . . .
G  . . ."""
    )
    observer = ObservationType.STATE.get_observation_generator(world)
    assert observer.shape == (5,)
    world.reset()
    world.step([Action.SOUTH])
    obs0 = observer.observe()
    assert all(obs0[:, 3] == 0.0) # Should be using constant or enum for gem index

    world.step([Action.SOUTH])
    obs1 = observer.observe()
    assert all(obs1[:, 3] == 1.0)

    world.step([Action.NORTH])
    obs1 = observer.observe()
    assert all(obs1[:, 3] == 1.0)


def test_retrieve_normalized_world_state():
    w = World.level(1)
    w.reset()
    state = w.get_state()
    generator = ObservationType.NORMALIZED_STATE.get_observation_generator(w)
    np_state = generator.observe()[0]
    res = generator.to_world_state(np_state)
    assert res == state


def test_retrieve_not_normalized_world_state():
    w = World.level(1)
    w.reset()
    state = w.get_state()
    generator = ObservationType.STATE.get_observation_generator(w)
    np_state = generator.observe()[0]
    res = generator.to_world_state(np_state)
    assert res == state


def test_observe_rgb_not_empty():
    world = World(
        """
S0 X  .  .
.  .  S1 G
.  X  .  ."""
    )
    observer = ObservationType.RGB_IMAGE.get_observation_generator(world)
    world.reset()
    image = observer.observe()
    assert image.max() > image.min()


def test_observe_layered_change_exits():
    world = World("S0 X . .")
    observer = Layered(world)

    assert world.exit_pos[0] == (0, 1)
    assert len(world.exit_pos) == 1

    world.exit_pos = [(0, 2), (0, 3)] # this is working but hasnt impl a ways to suppress the warning about it
    world.reset()
    # Exit positions are cached in the generator's static layer and only refreshed on
    # `reset()` (see ObservationGenerator.reset); code that edits world topology directly,
    # outside of LLE.reset(), must call it explicitly afterwards.
    observer.reset()
    obs = observer.observe()
    print(obs.shape)
    assert np.all(obs[:, observer.EXIT, 0, 2] == 1)
    assert np.all(obs[:, observer.EXIT, 0, 3] == 1)


def test_observe_layered_deactivated_laser():
    world = World(
        """
@ @ L0S @  @
@ .  .  .  @
@ X  .  S0 @
@ X  .  S1 @
@ @  @  @  @
"""
    )
    observer = Layered(world)
    world.reset()
    layers = observer.observe()
    LASERS_0_LAYER = observer.LASER_0
    LASERS_1_LAYER = observer.LASER_0 + 1
    # Laser source and laser beam in layer 0
    assert np.all(layers[:, LASERS_0_LAYER, 0, 2] == -1)
    assert np.all(layers[:, LASERS_0_LAYER, 1:4, 2] == 1)
    # Nothing in layer 1
    assert np.all(layers[:, LASERS_1_LAYER] == 0)

    # Now deactivate laser by blocking it
    world.step([Action.WEST, Action.STAY])
    layers = observer.observe()
    # Laser source and laser beam in layer 0
    assert np.all(layers[:, LASERS_0_LAYER, 0, 2] == -1)
    assert np.all(layers[:, LASERS_0_LAYER, 2:4, 2] == 0)
    # Nothing in layer 1
    assert np.all(layers[:, LASERS_1_LAYER] == 0)


def test_observe_layered_gems_walls():
    world = World(
        """
@ @ L0S @  @
@ .  .  .  @
@ X  G  S0 @
@ .  .  .  @
@ @  @  @  @
"""
    )
    observer = Layered(world)
    world.reset()
    layers = observer.observe()
    LASER_0_LAYER = observer.LASER_0
    WALL_LAYER = observer.WALL
    VOID_LAYER = observer.VOID
    GEM_LAYER = observer.GEM
    EXIT_LAYER = observer.EXIT

    for i, j, k in world.wall_pos:
        assert np.all(layers[:, WALL_LAYER, i, j, k] == 1)
    for gem in world.gems:
        i, j, k = gem.pos
        assert np.all(layers[:, GEM_LAYER, i, j, k] == 1)
    for i, j, k in world.exit_pos:
        assert np.all(layers[:, EXIT_LAYER, i, j, k] == 1)
    for laser in world.lasers:
        if laser.is_on:
            i, j, k = laser.pos
            assert np.all(layers[:, LASER_0_LAYER + laser.agent_id, i, j, k] == 1)
    for source in world.laser_sources:
        (i, j, k) = source.pos
        assert np.all(layers[:, LASER_0_LAYER + source.agent_id, i, j, k] == -1)
    assert np.all(layers[:, VOID_LAYER] == 0)


def test_observe_layered_lift_button():
    world = World(
        """
        S0 . TU0
        .  . X
        ;
        .  . TD0
        .  . .
        """
    )
    observer = Layered(world)
    world.reset()
    layers = observer.observe()

    lift_0 = observer.LIFT_0 + observer.group_index[0]
    assert np.all(layers[:, lift_0, 0, 2, 0] == 1.0)  # Up direction
    assert np.all(layers[:, lift_0, 0, 2, 1] == -1.0)  # Down direction
    # No lift on a plain floor tile
    assert np.all(layers[:, lift_0, 0, 1, 0] == 0.0)


def test_observe_layered_button():
    world = World(
        """
        S0 B0 X
        """
    )
    observer = Layered(world)
    world.reset()
    layers = observer.observe()

    button_0 = observer.BUTTON_0 + observer.group_index[0]
    assert np.all(layers[:, button_0, 0, 1, 0] == 1.0)
    assert np.all(layers[:, button_0, 0, 0, 0] == 0.0)


def test_observe_layered_lift_button_groups_are_distinct():
    world = World(
        """
        S0 TU0 B0  X
        .  TU2 B2  .
        ;
        .  .   .   .
        .  .   .   .
        """
    )
    observer = Layered(world)
    world.reset()
    layers = observer.observe()

    group_a, group_b = observer.group_index[0], observer.group_index[2]
    assert group_a != group_b
    # A lift and a button of the same group share the same group offset.
    assert layers[0, observer.LIFT_0 + group_a, 0, 1, 0] == 1.0
    assert layers[0, observer.BUTTON_0 + group_a, 0, 2, 0] == 1.0
    assert layers[0, observer.LIFT_0 + group_b, 1, 1, 0] == 1.0
    assert layers[0, observer.BUTTON_0 + group_b, 1, 2, 0] == 1.0
    # ... and do not bleed into the other group's channels.
    assert layers[0, observer.LIFT_0 + group_b, 0, 1, 0] == 0.0
    assert layers[0, observer.BUTTON_0 + group_b, 0, 2, 0] == 0.0


def test_observe_layered_authorization():
    world = World(
        """
        S0 B0A1 B0  X
        S1 .    TU0 X
        ;
        .  .    .   .
        .  .    .   .
        """
    )
    observer = Layered(world)
    world.reset()
    layers = observer.observe()

    # `B0A1` may only be actuated by agent 1.
    assert layers[0, observer.AUTH_0 + 0, 0, 1, 0] == 0.0
    assert layers[0, observer.AUTH_0 + 1, 0, 1, 0] == 1.0
    # An unrestricted button is usable by everyone.
    assert layers[0, observer.AUTH_0 + 0, 0, 2, 0] == 1.0
    assert layers[0, observer.AUTH_0 + 1, 0, 2, 0] == 1.0
    # Same for an unrestricted lift.
    assert layers[0, observer.AUTH_0 + 0, 1, 2, 0] == 1.0
    assert layers[0, observer.AUTH_0 + 1, 1, 2, 0] == 1.0
    # Nothing is flagged on a plain floor tile.
    assert np.all(layers[0, observer.AUTH_0 : observer.AUTH_0 + 2, 1, 1, 0] == 0.0)


def test_observe_layered_authorized_lift():
    world = World(
        """
        S0 TU0A1 X
        S1 .     X
        ;
        .  .     .
        .  .     .
        """
    )
    observer = Layered(world)
    world.reset()
    layers = observer.observe()

    assert layers[0, observer.AUTH_0 + 0, 0, 1, 0] == 0.0
    assert layers[0, observer.AUTH_0 + 1, 0, 1, 0] == 1.0


def test_observe_layered_no_lift_no_button_has_no_lift_section():
    world = World("S0 . X")
    observer = Layered(world)
    assert observer.n_groups == 0
    assert observer.shape[0] == 2 * world.n_agents + 4


def test_observe_layered_void():
    world = World(
        """
    V . . S0
    . . . .
    V V G X"""
    )
    observer = Layered(world)
    world.reset()
    layers = observer.observe()
    positions = [(0, 0), (2, 0), (2, 1)]
    for i in range(world.height):
        for j in range(world.width):
            if (i, j) in positions:
                assert np.all(layers[:, observer.VOID, i, j, 0] == 1.0)
            else:
                assert np.all(layers[:, observer.VOID, i, j, 0] == 0.0)


def test_observe_flattened():
    world = World(
        """
@ @ L0S @  @
@ .  .  .  @
@ X  G  S0 @
@ .  .  .  @
@ @  @  @  @
"""
    )
    observer = ObservationType.FLATTENED.get_observation_generator(world)
    #  4 layers: walls, gems, exits, voids (this world has no lift nor button)
    # +2 layer per agent: location, lasers
    assert observer.shape == (world.width * world.height * world.layers * (world.n_colours * 2 + 4),)
    world.reset()
    obs = observer.observe()
    assert obs.shape == (
        1,
        (world.n_colours * 2 + 4) * world.width * world.height * world.layers,
    )


def test_world_initial_observation():
    world = World(
        """S0 X .
.  . .
.  . ."""
    )
    observer = ObservationType.NORMALIZED_STATE.get_observation_generator(world)
    world.reset()
    obs0 = observer.observe()
    expected = np.array([[0.0, 0.0, 0.0, 1.0]])
    assert np.array_equal(expected, obs0)

    world = World(
        """
    S0 X  .
    .  .  S1
    .  .  X"""
    )
    observer = ObservationType.NORMALIZED_STATE.get_observation_generator(world)
    world.reset()
    obs0 = observer.observe()
    expected = np.tile(np.array([0.0, 0.0,0.0, 1 / 3, 2 / 3,0.0, 1.0, 1.0]), (2, 1))
    assert np.allclose(expected, obs0)

    world = World(
        """
S0 X  .  .
.  .  S1  .
.  X  .  ."""
    )
    observer = ObservationType.NORMALIZED_STATE.get_observation_generator(world)
    world.reset()
    obs0 = observer.observe()
    expected = np.tile(np.array([0.0, 0.0,0.0,  1 / 3, 1 / 2,0.0, 1.0, 1.0]), (2, 1))
    assert np.allclose(expected, obs0)

    world = World(
        """
S0 X  .  G
.  .  S1  .
.  X  .  ."""
    )
    observer = ObservationType.NORMALIZED_STATE.get_observation_generator(world)
    world.reset()
    obs0 = observer.observe()
    expected = np.tile(np.array([0.0, 0.0,0.0,  1 / 3, 1 / 2, 0.0 ,0.0, 1.0, 1.0]), (2, 1))
    assert np.allclose(expected, obs0)


def test_partial_3x3():
    world = World(
        """
    S0 X  @
    G  S1 @
    .  .  X"""
    )
    world.reset()

    observer = PartialGenerator(world, 3)
    obs0, obs1 = observer.observe()

    assert obs0[0, 1, 1] == 1
    assert obs0[1, 2, 2] == 1

    assert obs1[0, 0, 0] == 1
    assert obs1[1, 1, 1] == 1

    assert obs0[observer.GEM, 2, 1] == 1
    assert obs1[observer.GEM, 1, 0] == 1

    assert obs1[observer.EXIT, 2, 2] == 1

    assert np.all(obs0[observer.WALL] == 0)
    assert obs1[observer.WALL, 1, 2] == 1
    assert obs1[observer.WALL, 0, 2] == 1


def test_partial_7x7():
    world = World(
        """
S0 S1 S2 S3 X X X X
"""
    )
    world.reset()
    observer = PartialGenerator(world, 7)
    assert observer.shape[-2:] == (7, 7)
    center = 3
    observations = observer.observe()
    # Only check the observation of the first agent
    for agent_num, obs in enumerate(observations):
        for other_agent_num in range(world.n_agents):
            # Agents are side to side
            i = center
            j = center - agent_num + other_agent_num
            assert obs[other_agent_num, i, j] == 1
            # All other positions should be empty
            obs[other_agent_num, i, j] = 0
            assert np.all(obs[0] == 0)
    # Exits
    assert np.all(observations[0, observer.EXIT] == 0)
    assert observations[1, observer.EXIT, center, center + 3] == 1
    assert np.all(observations[2, observer.EXIT, center, center + 2 :] == 1)
    assert np.all(observations[3, observer.EXIT, center, center + 1 :] == 1)
    # Others
    assert np.all(observations[:, observer.WALL] == 0)
    assert np.all(observations[:, observer.GEM] == 0)
    assert np.all(observations[:, observer.LASER_0 : observer.LASER_0 + world.n_colours] == 0)


def test_partial_3x3_lasers():
    world = World(
        """
    .   L0S S1
    S0   .   .
    L1E  X   X
"""
    )
    world.reset()

    observer = PartialGenerator(world, 3)
    obs0, obs1 = observer.observe()

    assert obs0[observer.LASER_0, 0, 2] == -1
    assert obs0[observer.LASER_0, 1, 2] == 1
    assert obs0[observer.LASER_0, 2, 2] == 1

    assert obs0[observer.LASER_0 + 1, 2, 1] == -1
    assert obs0[observer.LASER_0 + 1, 2, 2] == 1


def test_partial_3x3_lift_button():
    world = World(
        """
        TU0 S0 B0
        .   .  .
        .   .  X
        """
    )
    world.reset()

    observer = PartialGenerator(world, 3)
    (obs0,) = observer.observe()

    assert obs0[observer.LIFT_0 + observer.group_index[0], 1, 0] == 1
    assert obs0[observer.BUTTON_0 + observer.group_index[0], 1, 2] == 1
    # Both tiles are unrestricted, so the only agent is authorized on both.
    assert obs0[observer.AUTH_0 + 0, 1, 0] == 1
    assert obs0[observer.AUTH_0 + 0, 1, 2] == 1


def test_partial_3x3_authorization():
    world = World(
        """
        B0A1 S0 B0A0
        S1   .  .
        X    .  X
        """
    )
    world.reset()

    observer = PartialGenerator(world, 3)
    obs0, _ = observer.observe()

    # Agent 0 sees that it may use the button on its right but not the one on its left.
    assert obs0[observer.AUTH_0 + 0, 1, 0] == 0
    assert obs0[observer.AUTH_0 + 1, 1, 0] == 1
    assert obs0[observer.AUTH_0 + 0, 1, 2] == 1
    assert obs0[observer.AUTH_0 + 1, 1, 2] == 0


def test_partial_3x3_ignores_other_floors():
    world = World(
        """
        S0 . X
        .  . .
        ;
        .  G .
        .  . .
        """
    )
    world.reset()

    observer = PartialGenerator(world, 3)
    (obs0,) = observer.observe()

    # The gem sits at (0, 1) on the floor below: it must not leak into the
    # window of an agent standing on floor 0.
    assert np.all(obs0[observer.GEM] == 0.0)


def test_padded_layered():
    world = World("S0 X")
    baseline = ObservationType.LAYERED.get_observation_generator(world)
    obs = ObservationType.LAYERED_PADDED_1AGENT.get_observation_generator(world)
    assert obs.shape[0] == baseline.shape[0] + 2
    assert obs.shape[1:] == baseline.shape[1:]
    obs = ObservationType.LAYERED_PADDED_2AGENTS.get_observation_generator(world)
    assert obs.shape[0] == baseline.shape[0] + 4
    assert obs.shape[1:] == baseline.shape[1:]
    obs = ObservationType.LAYERED_PADDED_3AGENTS.get_observation_generator(world)
    assert obs.shape[0] == baseline.shape[0] + 6
    assert obs.shape[1:] == baseline.shape[1:]


# Same geometry and agent count, only the number of lift/button groups differs.
_ZERO_GROUPS = """S0 .   .  X
                  S1 .   .  X
                  ;
                  .  .   .  .
                  .  .   .  ."""
_ONE_GROUP = """S0 TU0 B0 X
                S1 .   .  X
                ;
                .  .   .  .
                .  .   .  ."""
_TWO_GROUPS = """S0 TU0 B0 X
                 S1 TU1 B1 X
                 ;
                 .  .   .  .
                 .  .   .  ."""


def test_group_count_changes_shape_by_default():
    """Without a declared budget, the shape follows the map's group count."""
    shapes = {
        ObservationType.LAYERED.get_observation_generator(World(src)).shape[0]
        for src in (_ZERO_GROUPS, _ONE_GROUP, _TWO_GROUPS)
    }
    assert len(shapes) == 3


def test_declared_n_groups_pins_the_shape():
    """A declared budget makes maps with different group counts interchangeable."""
    for obs_type in (ObservationType.LAYERED, ObservationType.FLATTENED, ObservationType.PARTIAL_3x3):
        shapes = {
            obs_type.get_observation_generator(World(src), n_groups=3).shape
            for src in (_ZERO_GROUPS, _ONE_GROUP, _TWO_GROUPS)
        }
        assert len(shapes) == 1, f"{obs_type.name} shapes differ: {shapes}"


def test_declared_n_groups_reserves_empty_channels():
    world = World(_ONE_GROUP)
    observer = ObservationType.LAYERED.get_observation_generator(world, n_groups=3)
    assert observer.n_groups == 3
    layers = observer.observe()
    # The world only uses the first group, the two reserved ones stay empty.
    assert np.any(layers[0, observer.LIFT_0 + 0] != 0.0)
    assert np.all(layers[0, observer.LIFT_0 + 1 : observer.BUTTON_0] == 0.0)
    assert np.all(layers[0, observer.BUTTON_0 + 1 : observer.AUTH_0] == 0.0)


def test_declared_n_groups_on_a_world_without_lifts():
    """The section is emitted on a lift-free map so it stays shape-compatible."""
    observer = ObservationType.LAYERED.get_observation_generator(World(_ZERO_GROUPS), n_groups=2)
    layers = observer.observe()
    assert observer.shape[0] == 3 * 2 + 4 + 2 * 2
    assert np.all(layers[:, observer.LIFT_0 :] == 0.0)


def test_n_groups_too_small_raises():
    with pytest.raises(ValueError):
        ObservationType.LAYERED.get_observation_generator(World(_TWO_GROUPS), n_groups=1)
    with pytest.raises(ValueError):
        ObservationType.PARTIAL_3x3.get_observation_generator(World(_ONE_GROUP), n_groups=0)


def test_set_world_refreshes_static_layers():
    observer = ObservationType.LAYERED.get_observation_generator(World(_ONE_GROUP), n_groups=3)
    observer.set_world(World(_TWO_GROUPS))
    layers = observer.observe()

    # The second group of the new world is now visible...
    assert layers[0, observer.LIFT_0 + observer.group_index[1], 1, 1, 0] == 1.0
    assert layers[0, observer.BUTTON_0 + observer.group_index[1], 1, 2, 0] == 1.0
    # ... and so is its authorization block.
    assert np.all(layers[0, observer.AUTH_0 : observer.AUTH_0 + 2, 1, 1, 0] == 1.0)


def test_set_world_refreshes_walls():
    observer = ObservationType.LAYERED.get_observation_generator(World("S0 @ X"))
    assert observer.observe()[0, observer.WALL, 0, 1, 0] == 1.0
    observer.set_world(World("S0 . X"))
    assert observer.observe()[0, observer.WALL, 0, 1, 0] == 0.0


def test_set_world_rejects_a_world_that_would_reshape():
    observer = ObservationType.LAYERED.get_observation_generator(World(_ONE_GROUP))
    with pytest.raises(ValueError):  # more groups than reserved
        observer.set_world(World(_TWO_GROUPS))
    with pytest.raises(ValueError):  # different dimensions
        observer.set_world(World("S0 . X"))

    observer = ObservationType.LAYERED.get_observation_generator(World("S0 . X"))
    with pytest.raises(ValueError):  # more agents than reserved
        observer.set_world(World("S0 S1 X X"))


def test_set_world_partial_allows_other_dimensions():
    """The partial window is map-independent, so only the budgets have to hold."""
    observer = ObservationType.PARTIAL_3x3.get_observation_generator(World(_ONE_GROUP), n_groups=2)
    observer.set_world(World(_TWO_GROUPS))
    (obs0, _) = observer.observe()
    assert obs0[observer.LIFT_0 + observer.group_index[0], 1, 2] == 1.0


def test_perspective_is_centred_on_the_observing_agent():
    world = World("""
                  S0  S1 S2 X
                  L0E .  X  .
                   .  .  X L1W
                  """)
    world.reset()
    generator = PerspectiveLayered(world)
    obs = generator.observe()

    # Full observability on a canvas large enough to centre the agent wherever it stands.
    assert generator.shape == (world.n_colours * 2 + 4, 2 * world.height - 1, 2 * world.width - 1, world.layers)
    assert obs.shape == (world.n_agents, *generator.shape)

    centre_i, centre_j = world.height - 1, world.width - 1
    for num in range(world.n_agents):
        # Each agent sees itself at the centre, in the canonical colour-0 layer.
        assert obs[num, generator.A0, centre_i, centre_j, 0] == 1.0, f"Agent {num} is not at the centre"


def test_perspective_canonicalises_the_observing_agents_colour_to_zero():
    world = World("""
                  S0  S1 S2 X
                  L0E .  X  .
                   .  .  X L1W
                  """)
    world.reset()
    generator = PerspectiveLayered(world)
    obs = generator.observe()

    def shift(agent_num: int):
        i, j, _ = world.agents_positions[agent_num]
        return world.height - 1 - i, world.width - 1 - j

    # Agent 0 has colour 0: the permutation is the identity.
    di, dj = shift(0)
    assert obs[0][generator.LASER_0, 1 + di, 0 + dj] == -1.0
    assert obs[0][generator.LASER_0 + 1, 2 + di, 3 + dj] == -1.0
    # Agent 1 has colour 1: its own colour maps to 0, and colour 0 takes slot 1.
    di, dj = shift(1)
    assert obs[1][generator.LASER_0, 2 + di, 3 + dj] == -1.0
    assert obs[1][generator.LASER_0 + 1, 1 + di, 0 + dj] == -1.0


def test_perspective_same_colour_agents_share_their_layer():
    """The point of the whole change: agents of one colour stamp one layer, like lasers do.
    Identity is preserved by the centring, not by the layer index."""
    world = World("S0 S0 X X")
    world.reset()
    generator = PerspectiveLayered(world)
    obs = generator.observe()

    assert world.n_agents == 2
    centre = (world.height - 1, world.width - 1)
    for num in range(world.n_agents):
        agent_layer = obs[num][generator.A0]
        assert agent_layer.sum() == 2.0, "Both colour-0 agents appear in the colour-0 layer"
        assert agent_layer[centre] == 1.0, "The observing agent is the one at the centre"


def test_perspective_off_map_cells_are_walls():
    world = World("S0 X .\n.  . G")
    world.reset()
    generator = PerspectiveLayered(world)
    obs = generator.observe()[0]
    # Agent 0 sits at (0, 0), so everything north and west of the centre is outside the map.
    centre_i, centre_j = world.height - 1, world.width - 1
    assert obs[generator.WALL, centre_i - 1, centre_j] == 1.0
    assert obs[generator.WALL, centre_i, centre_j - 1] == 1.0
    assert obs[generator.WALL, centre_i, centre_j] == 0.0


def test_perspective_matches_layered_modulo_translation_and_colour_swap():
    """Replaces the old `test_perspective2`, which pinned the layer-swap definition: the view is
    now the layered view translated so the agent lands at the centre, with its colour swapped
    into slot 0."""
    world = World("""
                  S0  S1 S2
                   .   .  .
                  L0E  X  .
                  L1E  X  .
                  L2E  X  .
                  """)
    world.reset()
    baseline = Layered(world)
    generator = PerspectiveLayered(world)

    for actions in (None, [Action.SOUTH, Action.SOUTH, Action.SOUTH]):
        if actions is not None:
            world.step(actions)

        layered_obs = baseline.observe()
        perspective_obs = generator.observe()

        for observer, (i, j, _) in enumerate(world.agents_positions):
            colour = world.agents[observer].colour
            expected = np.copy(layered_obs[observer])
            expected[[generator.A0, generator.A0 + colour]] = expected[[generator.A0 + colour, generator.A0]]
            expected[[generator.LASER_0, generator.LASER_0 + colour]] = expected[
                [generator.LASER_0 + colour, generator.LASER_0]
            ]
            di, dj = world.height - 1 - i, world.width - 1 - j
            window = perspective_obs[observer][:, di : di + world.height, dj : dj + world.width]
            np.testing.assert_array_equal(window, expected)


def test_perspective_generator_reports_its_observation_type():
    generator = PerspectiveLayered(World("S0 X"))

    assert generator.obs_type is ObservationType.PERSPECTIVE


def test_perspective_cannot_reconstruct_a_world_state():
    world = World("S0 S0 X X")
    world.reset()
    generator = PerspectiveLayered(world)
    with pytest.raises(NotImplementedError):
        generator.to_world_state(generator.observe()[0])


def test_layered_agent_layers_are_keyed_by_colour():
    """Colours may be sparse without being shared: the agent band is sized by the colour space."""
    world = World("S0 S2 X X")
    world.reset()
    generator = Layered(world)
    assert world.n_colours == 3
    assert generator.shape == (world.n_colours * 2 + 4, world.height, world.width, world.layers)

    obs = generator.observe()[0]
    (i0, j0, k0), (i1, j1, k1) = world.agents_positions
    assert obs[generator.A0, i0, j0, k0] == 1.0
    assert obs[generator.A0 + 2, i1, j1, k1] == 1.0, "The colour-2 agent belongs in the colour-2 layer"
    assert np.all(obs[generator.A0 + 1] == 0.0), "No agent has colour 1"


def test_partial_layers_are_keyed_by_colour():
    world = World("S0 S2 X X")
    world.reset()
    generator = PartialGenerator(world, 3)
    assert generator.shape == (world.n_colours * 2 + 3, 3, 3)


@pytest.mark.parametrize(
    "obs_type",
    ["layered", "flattened", "partial3x3", "layered-padded-2"],
)
def test_non_perspective_generators_refuse_shared_colours(obs_type: str):
    """Once two agents share a colour, a non-centred observation cannot say which of them it is
    addressed to, so these generators refuse to be built (plan §5.2)."""
    world = World("S0 S0 X X")
    with pytest.raises(ValueError, match="perspective"):
        ObservationType.from_str(obs_type).get_observation_generator(world)


def test_perspective_carries_lift_button_authorization():
    world = World("""
                  S0 S1 B0A1 X
                  .  .  .    X
                  ;
                  .  .  TU0  .
                  .  .  .    .
                  """)
    world.reset()
    generator = PerspectiveLayered(world)
    obs = generator.observe()
    AUTH = generator.AUTH_0

    # The button is restricted to agent 1. The AUTH band is per-agent-identity, not
    # per-colour, so it is never permuted: every observer sees the same values, just
    # shifted into its own window.
    button_i, button_j, button_k = 0, 2, 0
    for num, (i, j, _) in enumerate(world.agents_positions):
        di, dj = world.height - 1 - i, world.width - 1 - j
        assert obs[num, AUTH + 0, button_i + di, button_j + dj, button_k] == 0
        assert obs[num, AUTH + 1, button_i + di, button_j + dj, button_k] == 1


def _perform_tests_extras_one_agent(env: LLE):
    assert env.extras_shape[0] == 1

    obs, _ = env.reset()
    assert obs.extras_shape[0] == 1
    assert obs.extras[0][0] == 0.0


def test_subgoal_extras_one_laser():
    env = (
        LLE.from_str("""
                       S0  X
                       .  L0W""")
        .add_extras("laser_subgoal")
        .build()
    )
    _perform_tests_extras_one_agent(env)
    env.reset()
    _perform_tests_extras_one_agent(env)


def test_pbrs_subgoals_extras_one_laser():
    env = (
        LLE.from_str("""
                       S0  X
                       .  L0W""")
        .pbrs(with_extras=True)
        .build()
    )
    _perform_tests_extras_one_agent(env)
    env.reset()
    _perform_tests_extras_one_agent(env)


def _perform_tests_two_agents(env: LLE):
    assert env.extras_shape[0] == 2

    obs, _ = env.reset()
    assert obs.extras_shape[0] == 2
    assert np.all(obs.extras == 0.0)

    step = env.step([Action.SOUTH.value, Action.STAY.value])
    obs = step.obs
    assert obs.extras_shape[0] == 2
    assert np.sum(obs.extras[0]) == 1.0
    assert np.sum(obs.extras[1]) == 0.0

    step = env.step([Action.NORTH.value, Action.STAY.value])
    obs = step.obs
    assert obs.extras_shape[0] == 2
    assert np.sum(obs.extras[0]) == 1.0
    assert np.sum(obs.extras[1]) == 0.0

    # Even when an agent dies, the subgoal is reached
    step = env.step([Action.STAY.value, Action.SOUTH.value])
    assert step.done
    obs = step.obs
    assert obs.extras_shape[0] == 2
    assert np.sum(obs.extras[0]) == 1.0
    assert np.sum(obs.extras[1]) == 1.0


def test_pbrs_subgoals_extras_two_lasers_two_agents():
    env = (
        LLE.from_str("""
                       S0  S1 X  X
                       .   .  . L0W
                       .   .  . L1W""")
        .pbrs(with_extras=True)
        .build()
    )
    _perform_tests_two_agents(env)
    env.reset()
    _perform_tests_two_agents(env)


def test_extras_subgoals_extras_two_lasers_two_agents():
    env = (
        LLE.from_str("""
                       S0  S1 X  X
                       .   .  . L0W
                       .   .  . L1W""")
        .add_extras("laser_subgoal")
        .build()
    )
    _perform_tests_two_agents(env)
    env.reset()
    _perform_tests_two_agents(env)


def test_laser_colour_above_n_agents_does_not_alias_into_the_wall_layer():
    """A laser colour beyond `n_agents` used to alias into the WALL band, because the LASER band
    was `n_agents` wide. Both bands are now sized by the colour space (plan §3.2, §5.4)."""
    world = World("S0 L1E X\n@  .   .")
    generator = Layered(world)
    assert world.n_colours == 2
    assert generator.shape == (world.n_colours * 2 + 4, world.height, world.width, world.layers)
    data = generator.observe()
    laser_1_layer = data[0, generator.LASER_0 + 1]
    assert laser_1_layer[0, 1] == -1
    assert laser_1_layer[0, 2] == 1
    # The wall is in the WALL layer, and nowhere near the laser band.
    assert data[0, generator.WALL, 1, 0] == 1
    assert laser_1_layer[1, 0] == 0


def test_all_shapes():
    for level in range(1, 7):
        world = World.level(level)
        for variant in ObservationType:
            observer = variant.get_observation_generator(world)
            obs = observer.observe()
            for agent_num in range(world.n_agents):
                assert obs[agent_num].shape == observer.shape, (
                    f"{variant.name} shape is not consistent: announced {observer.shape} but returned {obs[agent_num].shape}"
                )
