from lle import Direction


def test_equality():
    for d in [Direction.NORTH, Direction.SOUTH, Direction.EAST, Direction.WEST]:
        assert d == d
        for d2 in [Direction.NORTH, Direction.SOUTH, Direction.EAST, Direction.WEST]:
            if d is not d2:
                assert d != d2
