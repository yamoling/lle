from lle import World, WorldState, LLE, ObservationType
from serde.json import to_json
import json
import pickle
import random


def test_pickle_world_state():
    for i in range(50):
        s = WorldState(
            gems_collected=[
                random.choice([True, False]) for _ in range(random.randint(0, 10))
            ],
            agents_positions=[
                (random.randint(0, 50), random.randint(0, 90))
                for _ in range(random.randint(0, 10))
            ],
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
            deserialised = pickle.loads(serialised)
            assert world.get_state() == deserialised.get_state()
            i += 1


def test_lle_json():
    env = LLE.level(6, ObservationType.FLATTENED)
    data = to_json(env)
    as_dict = json.loads(data)
    assert as_dict["name"] == "LLE-lvl6"
    assert as_dict["n_agents"] == 4
    assert as_dict["n_actions"] == 5
    assert as_dict["obs_type"] == ObservationType.FLATTENED.name
