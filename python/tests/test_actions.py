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
