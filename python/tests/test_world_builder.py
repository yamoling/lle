from lle import WorldBuilder, Direction


def test_world_builder():
    b = WorldBuilder(10, 10, 2)
    assert b.n_agents == 2
    assert b.width == 10
    assert b.height == 10
    assert len(b.start_positions) == 0
    b.set_start((0, 0), 0)

    pos = b.start_positions.get(0)
    assert pos == (0, 0)

    b.set_start((0, 1), 1)

    pos = b.start_positions.get(1)
    assert pos == (0, 1)

    b.add_exit((9, 9))
    b.add_exit((9, 2))
    b.add_gem((5, 5))
    w = b.build()
    w.reset()
    assert w.n_agents == 2
    assert w.width == 10
    assert w.height == 10
    assert (9, 9) in w.exit_pos
    assert (9, 2) in w.exit_pos
    assert (5, 5) in w.gems

    assert (0, 0) in w.start_pos
    assert (0, 1) in w.start_pos


def test_world_builder_errors():
    b = WorldBuilder(10, 10, 2)
    b.set_start((0, 1), 1)
    b.set_start((0, 0), 0)
    try:
        b.set_start((5, 5), 3)
        assert False, "There are only two agents in the world, should not be able to add a third start position"
    except ValueError:
        pass


def test_add_in_occupied_position():
    b = WorldBuilder(10, 10, 4)
    b.set_start((0, 1), 1)
    try:
        b.add_exit((0, 1))
        assert False, "Should not be able to add an exit in an occupied position"
    except ValueError:
        pass

    try:
        b.add_gem((0, 1))
        assert False, "Should not be able to add a gem in an occupied position"
    except ValueError:
        pass

    try:
        b.add_wall((0, 1))
        assert False, "Should not be able to add a wall in an occupied position"
    except ValueError:
        pass

    try:
        b.add_laser_source((0, 1), 0, Direction.NORTH)
        assert False, "Should not be able to add a laser source in an occupied position"
    except ValueError:
        pass


def test_world_str():
    b = WorldBuilder(3, 1, 1)
    assert b.world_str().strip() == ". . ."

    b.set_start((0, 0), 0)
    assert b.world_str().strip() == "S0 . ."

    b.add_exit((0, 2))
    assert b.world_str().strip() == "S0 . X"

    b.add_gem((0, 1))
    assert b.world_str().strip() == "S0 G X"
