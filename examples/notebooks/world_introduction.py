import marimo

__generated_with = "0.23.9"
app = marimo.App()


@app.cell
def _():
    import marimo as mo

    return (mo,)


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    # Introduction to the Laser Learning Environment `World`
    This notebook goes through the `World` class and how to use it. The `World` class is meant to be used for low-level control of LLE, as opposed to the `LLE` class, meant for high-level control and multi-agent reinforcement learning.

    ## 1. How to install
    Enter the following instructions in a terminal:

    - `pip install laser-learning-environment`
    - `pip install matplotlib`

    This installs the Laser Learning Environment and its dependencies. After this operation, you should be able to import `lle` in this notebook.


    In order to display the environment, we will use some functions from the package `matplotlib`.
    """)
    return


@app.cell
def _():
    import matplotlib.pyplot as plt
    from lle import World

    DISPLAY = True

    def display_world(w: World):
        if DISPLAY:
            plt.imshow(w.get_image())
            plt.axis("off")
            plt.show()

    return World, display_world


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ## 2. Using LLE
    ### 2.1 Predefined levels
    There exist 6 predefined levels that you can directly access with `World.level(<level>)`.
    """)
    return


@app.cell
def _(World, display_world):
    world = World.level(5)
    display_world(world)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### 2.2 Create a custom board
    LLE works with files that define the environment. The cases are represented with characters in a quite intuitive manner, that allows to easily create a new board.

    | Character | Tile | Walkable | Comment |
    ------------|------|----------|---------|
    | `.` | Floor | Yes | The most basic tile. |
    | `@` | Wall  | No | A wall that blocks lasers. |
    | `X` | Exit  | Yes | An exit tile. The agent can no longer move after reaching it. |
    | `G` | Gem   | Yes | A gem to collect. |
    | `S<n>` | Start | Yes | Start position of agent `n`. |
    | `L<n><d>` | Laser source | No | Source of a laser of colour `n` (a number) beaming toward the direction `d` (N, S, E, W). |
    | `V` | Void | Yes | A void tile. The agent dies if it walks on it |

    #### 2.2.1 A simple empty board
    Let us define a $6\times 5$ board with one agent and one destination. Note that some board configurations are invalid. For instance, there must be at least one exit tile per agent.
    """)
    return


@app.cell
def _():
    def get_empty_board(x, y):
        return [["." for _ in range(x)] for _ in range(y)]

    def get_board_as_str(board):
        return "\n".join([" ".join(i) for i in board])

    empty_board = get_empty_board(6, 5)
    empty_board[0][0] = "S0"
    empty_board[4][5] = "X"
    return empty_board, get_board_as_str, get_empty_board


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    Let us now create a world from a text file by saving our board and then loading it into a world.
    """)
    return


@app.cell
def _(World, empty_board, get_board_as_str):
    from pathlib import Path

    boards_dir = Path("examples", "notebooks", "boards")

    def save_board(board, filename):
        """Saves a board to a file in order to reload it later"""
        with open(boards_dir / filename, "w") as f:
            f.write(get_board_as_str(board))

    save_board(empty_board, "empty")
    empty = World.from_file((boards_dir / "empty").as_posix())
    return boards_dir, empty, save_board


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    And directly display it using `matplotlib.pyplot.imshow()`:
    """)
    return


@app.cell
def _(display_world, empty):
    display_world(empty)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    #### 2.2.2 Some more complex boards
    We can use other characters as shown the the above table.

    For example, with walls and voids tiles:
    """)
    return


@app.cell
def _(World, boards_dir, display_world, get_empty_board, save_board):
    board_with_walls = get_empty_board(6, 5)
    board_with_walls[0][0] = "S0"
    board_with_walls[4][5] = "X"
    board_with_walls[1] = ["@", "@", "@", "@", "@", "."]
    board_with_walls[3] = [".", "V", "V", "V", "V", "V"]
    save_board(board_with_walls, "walls")
    with_walls = World.from_file(boards_dir / "walls")
    display_world(with_walls)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    Or with gems
    """)
    return


@app.cell
def _(World, boards_dir, display_world, get_empty_board, save_board):
    board_with_gems = get_empty_board(3, 3)
    board_with_gems[0][0] = "S0"
    board_with_gems[2][2] = "X"
    board_with_gems[2][0] = "G"
    board_with_gems[0][2] = "G"
    save_board(board_with_gems, "gems")
    with_gems = World.from_file(boards_dir / "gems")
    display_world(with_gems)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    Or, finally, with multiple agens:
    """)
    return


@app.cell
def _(World, boards_dir, display_world, get_empty_board, save_board):
    two_agents = get_empty_board(5, 5)
    two_agents[0][0] = "S0"
    two_agents[4][0] = "S1"
    two_agents[4][4] = "X"
    two_agents[0][4] = "X"
    save_board(two_agents, "two_agents")
    world_two_agents = World.from_file(boards_dir / "two_agents")
    display_world(world_two_agents)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### 2.3. Playing with lasers
    Lasers of a certain colors prevent agents of another color to pass. They can be blocked by agents of the same color. They form the basis of the constaints in the `LLE` framework.

    Lasers have two characteristics:
    - A number, which denotes the color of the laser
    - An orientation (North, South, West, East)

    As an example, a laser with color $0$ pointing to $East$ will be noted `L0E`.
    Here is an example of a two-agents-two-lasers game.
    """)
    return


@app.cell
def _(World, boards_dir, display_world, get_empty_board, save_board):
    tatl = get_empty_board(5, 5)
    tatl[0][0] = "S0"
    tatl[4][0] = "S1"
    tatl[4][4] = "X"
    tatl[0][4] = "X"
    tatl[0][1] = "L0S"
    tatl[4][3] = "L1N"
    save_board(tatl, "tatl")
    world_tatl = World.from_file(boards_dir / "tatl")
    display_world(world_tatl)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ## 2.4 Conclusion and final examples
    From now on, you should be familiar with the notion of board, agent, laser, gems, and wall. You should be able to create and display complex boards to represent complex problems.

    Remember: the whole point of this environment is to make the agents reach the targets, without passing through lasers of the wrong colors, and to collect as many gems as possible, in a minimum amount of time.

    This set-up describes a coordination problem.

    For the coming sections, we will work on different boards. A first simple linear one:
    """)
    return


@app.cell
def _(World, boards_dir, display_world, save_board):
    save_board([["S0", "G", "G", "X"]], "linear")
    linear = World.from_file(boards_dir / "linear")
    display_world(linear)
    return (linear,)


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    Then, a simple two-agents example:
    """)
    return


@app.cell
def _(World, display_world):
    board_linear = "\nS0 G . X\nS1 . G X\n"
    linear2 = World(board_linear)
    display_world(linear2)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    And finally a more complex example:
    """)
    return


@app.cell
def _(World, display_world):
    board = """
    S0  @   .  G  .   .  . G . .
    S1  @   .  .  .   .  . . . .
    .   @   .  .  .   G  . . . .
    .   @   .  . L0W  @  @ @ @ .
    .   @   .  .  .   .  . . V .
    .   @   @  @  @  L1S @ . V .
    .   .   .  .  .   .  @ . V .
    .   .   .  G  .   .  @ . V .
    .   .   .  G  .   .  @ . V X
    G  L0N  .  .  .   .  . . V X"""
    world_2 = World(board)
    display_world(world_2)
    return (world_2,)


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ## 3 Playing with agents
    ### 3.1 Moving agents
    To solve `LLE` problems, we have to move the agents across the board.
    This can be done using `Action` objects, which can be either North, South, East, West, or STAY.
    """)
    return


@app.cell
def _():
    from lle import Action

    return (Action,)


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    For example, we can apply two times the action `EAST` on the first agent in the simple linear case to complete the problem. First, we `reset` the world, then we apply the function `step`. It takes a `list` as parameter, with as many `Actions` as there are agents to move. It returns the list of events that happened during the step. Events have different types, described in the EventType enum.
    """)
    return


@app.cell
def _(display_world, linear):
    linear.reset()
    display_world(linear)
    return


@app.cell
def _(Action, display_world, linear):
    events = linear.step([Action.EAST])
    display_world(linear)
    print("Events:", events)
    return (events,)


@app.cell
def _(Action, display_world, events, linear):
    events = linear.step([Action.EAST])
    display_world(linear)
    print("Events :", events)
    return


@app.cell
def _(Action, display_world, linear):
    events_1 = linear.step([Action.EAST])
    display_world(linear)
    print("Events :", events_1)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    An agent that has reached an exit cannot move anymore and its only available action is `Action.STAY`.
    """)
    return


@app.cell
def _(linear):
    print(linear.agents[0].has_arrived)
    print("Available actions for agent 0:", linear.available_actions()[0])
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### 3.2 Getting information about the environment

    Some attributes of the world allow to get more information about the problem setting. We show some of them on this more complex example:
    """)
    return


@app.cell
def _(display_world, world_2):
    world_2.reset()
    display_world(world_2)
    return


@app.cell
def _(world_2):
    world_2.reset()
    print("Current state:", world_2.get_state())
    print("Number of gems:", world_2.n_gems)
    print("Number of collected gems:", world_2.gems_collected)
    print("Position of the walls:", world_2.wall_pos)
    print("Position of the voids:", world_2.void_pos)
    print("Position of the exits:", world_2.exit_pos)
    print("Position of the lasers:", world_2.lasers)
    print("Position of the agents:", world_2.agents_positions)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    An important method is the `available_actions` one, which gives the possible actions for each of the agents:
    """)
    return


@app.cell
def _(display_world, world_2):
    display_world(world_2)
    print("Available actions:", world_2.available_actions())
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    See how those actions change when we make a step:
    """)
    return


@app.cell
def _(Action, display_world, world_2):
    world_2.step([Action.STAY, Action.SOUTH])
    display_world(world_2)
    print("Available actions:", world_2.available_actions())
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    But see how some actions are not combinable: if we ask for two agents to reach the same tile, they do not move.
    """)
    return


@app.cell
def _(Action, display_world, world_2):
    world_2.step([Action.SOUTH, Action.NORTH])
    display_world(world_2)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ## 4. Conclusion
    In this notebook, we have seen how to design a `lle` problem, and we have shown how to interact with agents. We have defined the problem of solving such defined environments.
    """)
    return


if __name__ == "__main__":
    app.run()
