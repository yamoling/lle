from lle import World, WorldState, LLE
import pickle
import orjson
import random


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


def test_serialize_env_to_json():
    env = LLE.from_str("S0 L0E X").build()
    for key, value in env.__dict__.items():
        print(f"{key}: {value}")
    s = orjson.dumps(env, option=orjson.OPT_SERIALIZE_NUMPY)
    deserialized = orjson.loads(s)
    assert deserialized["n_agents"] == env.n_agents
    assert deserialized["obs_type"] == env.obs_type
    assert deserialized["state_type"] == env.state_type
    assert deserialized["walkable_lasers"] == env.walkable_lasers
    assert deserialized["randomize_lasers"] == env.randomize_lasers
