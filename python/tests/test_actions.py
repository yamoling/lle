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
