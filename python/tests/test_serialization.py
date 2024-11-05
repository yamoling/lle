from lle import World, WorldState
import pickle
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
    for pos in world.laser_sources:
        assert pos in deserialised.laser_sources
        assert world.laser_sources[pos].laser_id == deserialised.laser_sources[pos].laser_id
        assert world.laser_sources[pos].agent_id == deserialised.laser_sources[pos].agent_id
        assert world.laser_sources[pos].direction == deserialised.laser_sources[pos].direction
