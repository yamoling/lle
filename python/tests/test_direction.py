from lle import Direction


def test_equality():
    for d in [Direction.NORTH, Direction.SOUTH, Direction.EAST, Direction.WEST]:
        assert d == d
        for d2 in [Direction.NORTH, Direction.SOUTH, Direction.EAST, Direction.WEST]:
            if d is not d2:
                assert d != d2


def test_constructor():
    d = Direction("N")
    assert d == Direction.NORTH


def test_exception():
    try:
        Direction("z")  # type: ignore
        assert False, "Should raise an exception"
    except ValueError:
        pass


def test_delta():
    assert Direction.NORTH.delta() == (-1, 0)
    assert Direction.SOUTH.delta() == (1, 0)
    assert Direction.EAST.delta() == (0, 1)
    assert Direction.WEST.delta() == (0, -1)


def test_opposite():
    assert Direction.NORTH.opposite() == Direction.SOUTH
    assert Direction.SOUTH.opposite() == Direction.NORTH
    assert Direction.EAST.opposite() == Direction.WEST
    assert Direction.WEST.opposite() == Direction.EAST
