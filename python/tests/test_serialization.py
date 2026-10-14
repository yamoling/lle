import pickle
import random

import orjson
import pytest
from lle import LLE, Action, World, WorldState


def test_pickle_world_state():
    for i in range(50):
        s = WorldState(
            gems_collected=[random.choice([True, False]) for _ in range(random.randint(0, 10))],
            agents_positions=[(random.randint(0, 50), random.randint(0, 90)) for _ in range(random.randint(0, 10))],
        )
        serialised = pickle.dumps(s)
        deserialised = pickle.loads(serialised)
        assert s == deserialised


def test_pickle_world():
    for lvl in range(1, 7):
        world = World.level(lvl)
        world.reset()
        i = 0
        while i < 20:
            actions = [random.choice(a) for a in world.available_actions()]
            world.step(actions)
            serialised = pickle.dumps(world)
            deserialized = pickle.loads(serialised)
            assert deserialized.n_agents == world.n_agents
            assert deserialized.n_gems == world.n_gems
            assert deserialized.height == world.height
            assert deserialized.width == world.width
            assert deserialized.exit_pos == world.exit_pos
            assert deserialized.start_pos == world.start_pos
            assert deserialized.wall_pos == world.wall_pos
            assert deserialized.void_pos == world.void_pos
            assert world.get_state() == deserialized.get_state()
            i += 1


def test_pickled_world_keeps_same_laser_ids():
    world = World("L0E L1S S0 S1 X X")
    serialised = pickle.dumps(world)
    deserialised: World = pickle.loads(serialised)
    for source in world.laser_sources:
        pos = source.pos
        assert source in deserialised.laser_sources
        assert world.source_at(pos).laser_id == deserialised.source_at(pos).laser_id
        assert world.source_at(pos).agent_id == deserialised.source_at(pos).agent_id
        assert world.source_at(pos).direction == deserialised.source_at(pos).direction


def test_pickle_world_with_collected_gem_under_laser():
    """Pickling restores collected gems even when a laser wraps their tile."""
    world = World(". G X\nS0 L0N .")
    world.step(Action.NORTH)
    world.step(Action.EAST)
    assert world.get_state().gems_collected[0]
    deserialised = pickle.loads(pickle.dumps(world))
    assert deserialised.get_state() == world.get_state()


def test_serialize_env_to_json():
    env = LLE.from_str("S0 L0E X").build()
    s = orjson.dumps(env, option=orjson.OPT_SERIALIZE_NUMPY)
    deserialized = orjson.loads(s)
    assert deserialized["n_agents"] == env.n_agents
    assert deserialized["obs_type"] == env.obs_type
    assert deserialized["state_type"] == env.state_type
    assert deserialized["walkable_lasers"] == env.walkable_lasers
    assert deserialized["randomize_lasers"] == env.randomize_lasers


def test_world_state_without_boxes_is_backward_compatible():
    state = WorldState([(0, 1)], [])
    assert state.boxes_positions == []
    assert state.boxes_present == []


def test_world_state_with_boxes():
    state = WorldState([(0, 1)], [], [True], [(0, 2)], [True])
    assert state.boxes_positions == [(0, 2)]
    assert state.boxes_present == [True]


def test_world_state_with_boxes_pickles():
    state = WorldState([(0, 1)], [False], [True], [(0, 2)], [False])
    assert pickle.loads(pickle.dumps(state)) == state


def test_world_state_boxes_take_part_in_equality():
    a = WorldState([(0, 1)], [], [True], [(0, 2)], [True])
    assert a != WorldState([(0, 1)], [], [True], [(0, 3)], [True])
    assert a != WorldState([(0, 1)], [], [True], [(0, 2)], [False])
    assert a == WorldState([(0, 1)], [], [True], [(0, 2)], [True])


def test_world_state_array_round_trip_with_boxes():
    state = WorldState([(0, 1)], [True], [True], [(0, 2)], [True])
    array = state.as_array().tolist()
    assert len(array) == 1 * WorldState.AGENT_SIZE + 1 + 1 * WorldState.BOX_SIZE
    assert WorldState.from_array(array, n_agents=1, n_gems=1, n_boxes=1) == state


def test_world_state_array_round_trip_with_a_destroyed_box():
    state = WorldState([(2, 1), (0, 3)], [True, False], [True, True], [(4, 5), (1, 2)], [False, True])
    array = state.as_array().tolist()
    assert WorldState.from_array(array, n_agents=2, n_gems=2, n_boxes=2) == state


def test_from_array_defaults_to_no_boxes():
    state = WorldState([(0, 1)], [True], [True])
    assert WorldState.from_array(state.as_array().tolist(), 1, 1) == state


def test_from_array_rejects_a_wrong_length_with_boxes():
    with pytest.raises(ValueError):
        WorldState.from_array([0.0] * 6, n_agents=1, n_gems=1, n_boxes=1)


def test_pickle_world_state_after_a_push():
    world = World("S0 # . X")
    world.reset()
    world.step(Action.EAST)
    state = world.get_state()
    assert pickle.loads(pickle.dumps(state)) == state


def test_pickle_world_state_after_a_void_destruction():
    world = World("S0 # V X")
    world.reset()
    world.step(Action.EAST)
    state = world.get_state()
    restored = pickle.loads(pickle.dumps(state))
    assert restored == state
    assert restored.boxes_present == [False]


def test_pickle_world_after_a_push():
    world = World("S0 # . X")
    world.reset()
    world.step(Action.EAST)
    restored = pickle.loads(pickle.dumps(world))
    assert restored.get_state() == world.get_state()
    assert restored.boxes_positions == [(0, 2)]


def test_pickle_world_after_a_void_destruction():
    world = World("S0 # V X")
    world.reset()
    world.step(Action.EAST)
    restored = pickle.loads(pickle.dumps(world))
    assert restored.get_state() == world.get_state()
    assert restored.get_state().boxes_present == [False]


def test_state_generator_shape_matches_the_state_array():
    from lle.observations import StateGenerator

    world = World("S0 # G X")
    world.reset()
    generator = StateGenerator(world, normalize=False)
    assert generator.shape == (world.get_state().as_array().shape[0],)


def test_state_generator_round_trips_on_a_box_world():
    from lle.observations import StateGenerator

    for normalize in (False, True):
        world = World("S0 # G X")
        world.reset()
        world.step(Action.EAST)
        generator = StateGenerator(world, normalize=normalize)
        obs = generator.observe()[0]
        assert obs.shape == generator.shape
        assert generator.to_world_state(obs.copy()) == world.get_state()


@pytest.mark.parametrize("under", ["G", "X"])
def test_pickle_world_with_a_box_on_a_gem_or_an_exit(under: str):
    toml = f'world_string = """\nS0 {under} . X\n"""\n[[boxes]]\ni = 0\nj = 1\n'
    world = World(toml)
    world.reset()
    restored = pickle.loads(pickle.dumps(world))
    assert restored.boxes_positions == world.boxes_positions == [(0, 1)]
    assert sorted(g.pos for g in restored.gems) == sorted(g.pos for g in world.gems)
    assert restored.exit_pos == world.exit_pos
    assert restored.n_gems == world.n_gems
