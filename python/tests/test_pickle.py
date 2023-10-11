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
        while i < 20 and not world.done:
            actions = [random.choice(a) for a in world.available_actions()]
            world.step(actions)
            serialised = pickle.dumps(world)
            deserialised = pickle.loads(serialised)
            assert world.get_state() == deserialised.get_state()
            i += 1
