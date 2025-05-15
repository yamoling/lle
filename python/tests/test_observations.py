import numpy as np

from lle import World, Action, ObservationType, LLE
from lle.observations import PartialGenerator, Layered, AgentZeroPerspective


def test_observation_gem_collected():
    world = World(
        """
S0 X . .
.  . . .
G  . . ."""
    )
    observer = ObservationType.STATE.get_observation_generator(world)
    world.reset()
    world.step([Action.SOUTH])
    obs0 = observer.observe()
    assert all(obs0[:, 2] == 0.0)

    world.step([Action.SOUTH])
    obs1 = observer.observe()
    assert all(obs1[:, 2] == 1.0)

    world.step([Action.NORTH])
    obs1 = observer.observe()
    assert all(obs1[:, 2] == 1.0)


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

    world.exit_pos = [(0, 2), (0, 3)]
    world.reset()
    obs = observer.observe()

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
    observer = ObservationType.LAYERED.get_observation_generator(world)
    world.reset()
    layers = observer.observe()
    LASERS_0_LAYER = world.n_agents + 1
    LASERS_1_LAYER = LASERS_0_LAYER + 1
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
    observer = ObservationType.LAYERED.get_observation_generator(world)
    world.reset()
    layers = observer.observe()
    LASER_0_LAYER = world.n_agents + 1
    WALL_LAYER = world.n_agents
    VOID_LAYER = world.n_agents * 2 + 1
    GEM_LAYER = VOID_LAYER + 1
    EXIT_LAYER = -1

    for i, j in world.wall_pos:
        assert np.all(layers[:, WALL_LAYER, i, j] == 1)
    for gem in world.gems:
        i, j = gem.pos
        assert np.all(layers[:, GEM_LAYER, i, j] == 1)
    for i, j in world.exit_pos:
        assert np.all(layers[:, EXIT_LAYER, i, j] == 1)
    for laser in world.lasers:
        if laser.is_on:
            i, j = laser.pos
            assert np.all(layers[:, LASER_0_LAYER + laser.agent_id, i, j] == 1)
    for source in world.laser_sources:
        (i, j) = source.pos
        assert np.all(layers[:, LASER_0_LAYER + source.agent_id, i, j] == -1)
    assert np.all(layers[:, VOID_LAYER] == 0)


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
                assert np.all(layers[:, observer.VOID, i, j] == 1.0)
            else:
                assert np.all(layers[:, observer.VOID, i, j] == 0.0)


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
    assert observer.shape == (5 * 5 * (world.n_agents * 2 + 4),)
    world.reset()
    obs = observer.observe()
    assert obs.shape == (
        1,
        (world.n_agents * 2 + 4) * 5 * 5,
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
    expected = np.array([[0.0, 0.0, 1.0]])
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
    expected = np.tile(np.array([0.0, 0.0, 1 / 3, 2 / 3, 1.0, 1.0]), (2, 1))
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
    expected = np.tile(np.array([0.0, 0.0, 1 / 3, 1 / 2, 1.0, 1.0]), (2, 1))
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
    expected = np.tile(np.array([0.0, 0.0, 1 / 3, 1 / 2, 0.0, 1.0, 1.0]), (2, 1))
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
    assert np.all(observations[:, observer.LASER_0 : observer.LASER_0 + world.n_agents] == 0)


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


def test_padded_layered():
    world = World("S0 X")
    baseline = ObservationType.LAYERED.get_observation_generator(world)
    obs = ObservationType.LAYERED_PADDED_1AGENT.get_observation_generator(world)
    assert obs.shape[0] == baseline.shape[0] + 1
    assert obs.shape[1:] == baseline.shape[1:]
    obs = ObservationType.LAYERED_PADDED_2AGENTS.get_observation_generator(world)
    assert obs.shape[0] == baseline.shape[0] + 2
    assert obs.shape[1:] == baseline.shape[1:]
    obs = ObservationType.LAYERED_PADDED_3AGENTS.get_observation_generator(world)
    assert obs.shape[0] == baseline.shape[0] + 3
    assert obs.shape[1:] == baseline.shape[1:]


def test_perspective():
    world = World("""
                  S0  S1 S2 X
                  L0E .  X  .
                   .  .  X L1W
                  """)
    world.reset()
    generator = AgentZeroPerspective(world)
    A0 = generator.A0
    L0 = generator.LASER_0
    obs = generator.observe()

    assert obs.shape == (3, *generator.shape)
    obs0 = obs[0]
    obs1 = obs[1]
    obs2 = obs[2]

    assert obs0[A0, 0, 0] == 1
    assert obs1[A0, 0, 1] == 1
    assert obs2[A0, 0, 2] == 1

    assert obs0[L0, 1, 0] == -1
    assert np.all(obs0[L0, 1, 1:] == 1)

    assert obs1[L0, 2, 3] == -1
    assert np.all(obs1[L0, 2, :3] == 1)

    assert np.all(obs2[L0] == 0)


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


def test_layered_observation_laser_agent_id_gap():
    world = World("S0 L0S L2E X")
    generator = Layered(world)
    assert generator.highest_laser_agent_id == 2


def test_layered_observation_laser_source_agent_id_above_n_agents():
    world = World("S0 L1E X")
    generator = Layered(world)
    data = generator.observe()
    laser_1_layer = data[0, generator.LASER_0 + 1]
    assert laser_1_layer[0, 1] == -1
    assert laser_1_layer[0, 2] == 1
