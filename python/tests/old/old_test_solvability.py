from lle.solvability import make_graph
from lle.world import World
from lle import exceptions


def test_make_graph():
    map_content = """
    .  . G
    S0 . .
    . .  X
    """
    world = World.from_str(map_content)
    g, *_ = make_graph(world._grid)
    assert g.number_of_nodes() == 9
    assert g.number_of_edges() == 22


def test_not_solvable_corner():
    map_content = """
    G @  .
    @ S0 X
    """
    world = World.from_str(map_content)
    try:
        world.check_solvable()
        assert False, "Should have raised an exception"
    except exceptions.UnreachableGem:
        pass
    map_content = """
    X @  .
    @ S0 G
    """
    world = World.from_str(map_content)
    try:
        world.check_solvable()
        assert False, "Should have raised an exception"
    except exceptions.UnreachableFinish:
        pass


def test_obvious_solvable():
    map_content = """
    G .  .
    . S0 X
    """
    world = World.from_str(map_content)
    world.reset()
    world.check_solvable()


def test_laser_solvable():
    map_content = """
     G  . X
    L0N . .
    S0  . .
    """
    world = World.from_str(map_content)
    world.check_solvable()

    map_content = """
     G  . X
    L0E . .
    S0  . .
    """
    world = World.from_str(map_content)
    world.check_solvable()

    map_content = """
     G  . X  X
    L0E . .  .
    .   . .  L1W
    S0  . S1 .
    """
    world = World.from_str(map_content)
    world.check_solvable()


def test_laser_not_solvable():
    map_content = """
     G  . X  X
    L0E . .  .
    L1E . .  .
    S0  . S1 .
    """
    world = World.from_str(map_content)
    try:
        world.check_solvable()
        assert False, "Should have raised an exception"
    except exceptions.LevelMightBeUnfeasible:
        pass


def test_end_tile_blocks_not_solvable():
    map_content = """
    G  X .  
    @  . . 
    S0 . . 
    """
    world = World.from_str(map_content)
    try:
        world.check_solvable()
        assert False
    except exceptions.UnreachableGem:
        pass

    map_content = """
    G  X .  
    X  . . 
    S0 . S1 
    """
    world = World.from_str(map_content)
    try:
        world.check_solvable()
        assert False
    except exceptions.UnreachableGem:
        pass


def test_opposite_lasers_kill_on_spawn():
    map_content = """
    . @ G G . . .
    . . . . . @ G
    S0 X . G . . .
    . . . X . . .
    L1E S1 . G . . L0W
    . . . . . . .
    . . . . . . .
    """
    try:
        World.from_str(map_content)
        assert False, "Should have raised an exception"
    except exceptions.LaserSourceKillsAgentOnStart:
        pass


def test_4agents_solvable():
    map_content = """
    X . X G X
    S1 S2 L0S G .
    G . . . S0
    . @ @ @ G
    X S3 . @ G"""
    world = World.from_str(map_content)
    world.check_solvable()
