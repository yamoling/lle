from lle import Action


def test_equality_enum():
    a1 = Action.NORTH
    a2 = Action.NORTH
    assert a1 == a2


def test_equality_init():
    a1 = Action(0)
    a2 = Action(0)
    assert a1 == a2


def test_count():
    values = [Action.NORTH, Action.SOUTH, Action.EAST, Action.WEST, Action.WEST]
    assert values.count(Action.NORTH) == 1
    assert values.count(Action.SOUTH) == 1
    assert values.count(Action.EAST) == 1
    assert values.count(Action.WEST) == 2


def test_action_names():
    names = {a.name for a in Action.variants()}
    expected_names = {"NORTH", "SOUTH", "EAST", "WEST", "STAY"}
    assert names == expected_names


def test_hash_equal():
    hashes = set()
    actions = set()
    for a in Action.variants():
        h = hash(a)
        # deterministic
        assert h == hash(a)
        hashes.add(h)
        actions.add(a)


def test_deepcopy():
    import copy

    for a in Action.variants():
        b = copy.deepcopy(a)
        assert a == b


def test_pickle():
    import pickle

    for a in Action.variants():
        b = pickle.loads(pickle.dumps(a))
        assert a == b
        assert a is not b


def test_from_delta_stay():
    """Test from_delta with (0, 0) returns STAY"""
    action = Action.from_delta(0, 0)
    assert action == Action.STAY


def test_from_delta_north():
    """Test from_delta with (0, -1) returns NORTH"""
    action = Action.from_delta(0, -1)
    assert action == Action.NORTH


def test_from_delta_south():
    """Test from_delta with (0, 1) returns SOUTH"""
    action = Action.from_delta(0, 1)
    assert action == Action.SOUTH


def test_from_delta_east():
    """Test from_delta with (1, 0) returns EAST"""
    action = Action.from_delta(1, 0)
    assert action == Action.EAST


def test_from_delta_west():
    """Test from_delta with (-1, 0) returns WEST"""
    action = Action.from_delta(-1, 0)
    assert action == Action.WEST


def test_from_delta_invalid_values():
    """Test from_delta with invalid delta values raises PyValueError"""
    import pytest

    # Test various invalid delta combinations
    invalid_deltas = [
        (2, 0),  # Too far in x
        (-2, 0),  # Too far in x (negative)
        (0, 2),  # Too far in y
        (0, -2),  # Too far in y (negative)
        (1, 1),  # Diagonal
        (-1, 1),  # Diagonal
        (1, -1),  # Diagonal
        (-1, -1),  # Diagonal
        (2, 2),  # Large diagonal
        (5, 0),  # Very far
    ]

    for dx, dy in invalid_deltas:
        with pytest.raises(ValueError) as exc_info:
            Action.from_delta(dx, dy)
        assert f"Invalid delta: ({dx}, {dy})" in str(exc_info.value)
