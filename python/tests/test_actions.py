from lle import Action


def test_equality_enum():
    a1 = Action.NORTH
    a2 = Action.NORTH
    assert a1 == a2
    print()


def test_equality_init():
    a1 = Action(0)
    a2 = Action(0)
    assert a1 == a2
    print()


def test_count():
    values = [Action.NORTH, Action.SOUTH, Action.EAST, Action.WEST, Action.WEST]
    assert values.count(Action.NORTH) == 1
    assert values.count(Action.SOUTH) == 1
    assert values.count(Action.EAST) == 1
    assert values.count(Action.WEST) == 2


def test_hash_equal():
    hashes = set()
    actions = set()
    for a in Action.ALL:
        h = hash(a)
        # deterministic
        assert h == hash(a)
        hashes.add(h)
        actions.add(a)


def test_deepcopy():
    import copy

    for a in Action.ALL:
        b = copy.deepcopy(a)
        assert a == b
        assert a is not b


def test_pickle():
    import pickle

    for a in Action.ALL:
        b = pickle.loads(pickle.dumps(a))
        assert a == b
        assert a is not b
