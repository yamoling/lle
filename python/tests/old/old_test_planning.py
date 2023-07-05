import numpy as np
from lle import World, Action
from lle.planning import ProblemState, SearchProblem, breadth_first_search, heuristic, astar, get_difficulty, Difficulty


def test_start_state():
    world = World(
        """
    . . G
    S0 . .
    . . F
    """
    )
    p = SearchProblem(world)
    start = p.start_state
    assert len(start.agent_locations) == 1
    assert np.all(start.agent_locations == np.array([(1, 0)]))
    assert start.gems_collected == [False]


def test_goal_state():
    world = World(
        """
    . . G
    S0 . .
    . . F
    """
    )
    p = SearchProblem(world)
    s1 = ProblemState([(1, 1)], [False])
    assert p.is_goal_state(s1) == False
    s2 = ProblemState([(1, 1)], [True])
    assert p.is_goal_state(s2) == False
    s3 = ProblemState([(2, 2)], [True])
    assert p.is_goal_state(s3) == True


def test_get_successors_basic():
    world = World(
        """
    . . .
    S0 . .
    . . F
    """
    )
    p = SearchProblem(world)
    start = p.start_state
    successors = list(state for state, *_ in p.get_successors(start))
    expected_successors = [
        ProblemState([(0, 0)], []),
        ProblemState([(2, 0)], []),
        ProblemState([(1, 1)], []),
    ]
    assert len(successors) == len(expected_successors)
    for s in expected_successors:
        assert s in successors
        successors.pop(successors.index(s))
    assert len(successors) == 0


def test_get_successors_gem():
    world = World(
        """
    . .  .
    G S0 .
    . .  F
    """
    )
    p = SearchProblem(world)
    successors = list(state for state, *_ in p.get_successors(p.start_state))
    expected_successors = [
        ProblemState([(0, 1)], [False]),  # North
        ProblemState([(1, 0)], [True]),  # West
        ProblemState([(1, 2)], [False]),  # East
        ProblemState([(2, 1)], [False]),  # South
    ]
    assert len(successors) == len(expected_successors)
    for s in expected_successors:
        assert s in successors
        successors.pop(successors.index(s))
    assert len(successors) == 0


def test_successors_two_agents():
    world = World(
        """
    S1 . .
    S0 . F
    .  . F
    """
    )
    p = SearchProblem(world)
    successors = list(state for state, *_ in p.get_successors(p.start_state))
    expected_successors = [
        ProblemState([(1, 0), (0, 1)], []),  # NOOP - East
        ProblemState([(1, 1), (0, 1)], []),  # East -East
        ProblemState([(1, 1), (0, 0)], []),  # East - NOOP
        ProblemState([(2, 0), (0, 1)], []),  # South - East
        ProblemState([(2, 0), (0, 0)], []),  # South - NOOP
    ]
    assert len(successors) == len(expected_successors)
    for s in expected_successors:
        assert s in successors
        successors.pop(successors.index(s))
    assert len(successors) == 0


def test_get_successors_end():
    world = World(
        """
    . . .
    S0 . .
    . . F
    """
    )
    p = SearchProblem(world)
    end_state = ProblemState([(2, 2)], [])
    successors = list(p.get_successors(end_state))
    assert len(successors) == 0


def test_bfs_one_agent():
    world = World(
        """
    .  . .
    S0 . F
    .  . .
    """
    )
    p = SearchProblem(world)
    state_path, action_path = breadth_first_search(p)
    assert len(action_path) == 2 and len(state_path) == 2
    assert all(a == (Action.EAST,) for a in action_path)


def test_bfs_two_agents():
    world = World(
        """
    S1 . .
    S0 . F
    .  . F
    """
    )
    p = SearchProblem(world)
    state_path, action_path = breadth_first_search(p)
    assert len(state_path) == 3 and len(action_path) == 3


def test_bfs_impossible():
    world = World(
        """
    .  @ .
    S0 @ F
    .  @ .
    """
    )
    p = SearchProblem(world)
    path, actions = breadth_first_search(p)
    assert path is None and actions is None


def test_heuristic():
    world = World(
        """
    .  @ .
    S0 @ F
    .  @ .
    """
    )
    p = SearchProblem(world)
    heuristic(p.start_state, p) == 1
    world = World(
        """
    .  . G
    S0 . F
    .  G .
    """
    )
    p = SearchProblem(world)
    heuristic(p.start_state, p) == 3


def test_astar_path():
    world = World(
        """
    .  G .
    S0 @ F
    .  . .
    """
    )
    p = SearchProblem(world)
    states, actions = astar(p)
    assert len(actions) == 4
    assert actions[0][0] == Action.NORTH
    assert actions[1][0] == Action.EAST
    assert actions[2][0] == Action.EAST
    assert actions[3][0] == Action.SOUTH


def test_astar_no_path():
    world = World(
        """
    .  @ G
    S0 @ F
    .  @ .
    """
    )
    p = SearchProblem(world)
    states, actions = astar(p)
    assert states is None and actions is None


def test_difficulty_very_easy():
    world = World(
        """
    .  G .
    S0 @ F
    .  . .
    """
    )
    p = SearchProblem(world)
    states, _ = astar(p)
    level = get_difficulty(world)
    assert level == Difficulty.VERY_EASY


def test_difficulty_easy():
    world = World(
        """
    .  L0S .
    S0  .  F
    .   .  .
    """
    )
    level = get_difficulty(world)
    assert level == Difficulty.EASY

    world = World(". G L0E F @\n. . @ G .\n. . . . S1\n. L1E F G .\nS0 @ G . G")
    _, actions = astar(SearchProblem(world))
    assert get_difficulty(world) == Difficulty.EASY


def test_difficulty_medium():
    world = World(
        """
    .  L0S .
    S0  .  F
    S1  .  F
    """
    )
    level = get_difficulty(world)
    assert level == Difficulty.MEDIUM


def test_difficulty_hard():
    world = World(
        """
    .  L0S  .  F
    .   .   .  .
    S0  .   .  .
    S1  .  L1N F
    """
    )
    level = get_difficulty(world)
    assert level == Difficulty.HARD


def test_difficulty_unsolvable():
    world = World(
        """
    .  L0S  .  F
    .   @   .  .
    S0  .   @  .
    S1  .  L1N F
    """
    )
    level = get_difficulty(world)
    assert level == Difficulty.UNSOLVABLE

    world = World("G @ . . G\n. F G L0S S0\n. . G . .\n. @ L1E F @\n. . S1 G .")
    level = get_difficulty(world)
    assert level == Difficulty.UNSOLVABLE


def test_finish_tile_laser_kills_agent():
    world = World(
        """
. L0S S1 . @
F . G . .
L1N . @ G G
G . @ S0 .
. F . G .
    """
    )
    states, actions = astar(SearchProblem(world))
    assert states is None and actions is None
