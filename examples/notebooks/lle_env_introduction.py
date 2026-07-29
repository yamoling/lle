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
    # Introduction to the Laser Learning Environment (LLE)
    `lle` is a python package aimed at multi-agent reinforcement learning. The notebook gives an overview of the `LLE` class, designed for that purpose.
    The `LLE` class complies with the `RLEnv` *interface* from the `rlenv` package.

    A more fine-grained control can be obtained by using the `World` class.

    ## 1. How to install
    To use this notebook, install LLE and matplotlib with
    ```bash
    pip install laser-learning-environment
    pip install matplotlib
    ```

    After this operation, you should be able to import `lle` in this notebook:
    """)
    return


@app.cell
def _():
    import lle

    return (lle,)


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    If it works, you are ready to go !

    In order to display the environment, we will use some functions from the package `matplotlib`. Some other available examples use the package `cv2`. Change the variable $display$ to $False$ if you do not want this notebook to display the boards.
    """)
    return


@app.cell
def _():
    import matplotlib.pyplot as plt

    return (plt,)


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ## 2. Loading maps
    Due to the wide number of arguments (single or multi-objective, death strategy, ...) and the multiple instanciation methods (from strings, files or predefined levels), LLE uses the builder design pattern.

    ### 2.1 Standard levels
    LLE defines standard levels from 1 to 6 that can be loaded with `LLE.level(n)` and chained with `single_objective()` to load a single objective level 1 instance of LLE.
    """)
    return


@app.cell
def _(lle, plt):
    from marlenv import MARLEnv

    def display(env: MARLEnv):
        plt.imshow(env.get_image())
        plt.axis("off")
        plt.show()

    for lvl in [4, 5, 6]:
        env = lle.level(lvl).build()
        display(env)
    return display, env


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### 2.2 Custom boards
    For the sake of this notebook, we create a small custom board.
    Refer to the notebook `world_introduction.pynb` for more details on how to create your own boards.
    """)
    return


@app.cell
def _(display, lle):
    env_1 = lle.from_str("\nS0 . . G\n @ . . L0W\n . G . S1\n X . . X\n").build()
    display(env_1)
    return (env_1,)


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ## 3. Playing the game
    The objective of LLE is for the team of agents to collect all the gems on the board then reach the exit tiles. When an agent has reached an exit, it can not move anymore. The game ends when all the agents have reached an exit.

    ### 3.1 Observations
    An observation of the environment refers to the information that agents receive as input. An observation contains:
    - the actual observation data as a numpy array (one per agent)
    - some extras such as agent ids, time step, previous actions, etc. (one per agent, nothing in this case)
    - the state of the environment as a numpy array (useful for algorithms such as QMix)
    - the available actions (one-hot encoding)
    """)
    return


@app.cell
def _(env_1):
    def reset_example():
        obs, state = env_1.reset()
        print("Observation shape", obs.shape)
        print("Extras shape", obs.extras_shape)
        print("State shape", state.shape)
        print("Available actions:\n", obs.available_actions)

    reset_example()
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    #### Extras
    We could also add some extra information to the observation such as the ID of each agent and a time limit of 20 steps. For this, we can use the `rlenv.Builder`. This does not change the way the environment is rendered, but adds extra information to the observation (`agent_id`) or changes the behaviour of the environment (`time_limit`).
    """)
    return


@app.cell
def _(display, lle):
    import marlenv


    env_2 = (
        marlenv.Builder(lle.from_str("\nS0 . . G\n @ . . L0W\n . G . S1\n X . . X\n").build()).agent_id().time_limit(5).build()
    )
    display(env_2)
    _obs, _state = env_2.reset()
    print("Observation shape", _obs.shape)
    print("Extras with agent ID", _obs.extras)
    print("State shape", _state.shape)
    print("Available actions:\n", _obs.available_actions)
    return (env_2,)


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### 3.2 Actions
    The action space is discrete and there are 5 actions available defined in `Action.py`: North, South, East, West and Stay.
    When using the `LLE` class, actions should be referred to as integers from 0 to 4, where the integer refers to the value of the action in the `Action` enum class.
    """)
    return


@app.cell
def _(display, env_2):
    from lle import Action

    _step = env_2.step([Action.EAST.value, Action.WEST.value])
    display(env_2)
    return (Action,)


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    #### Action loop
    """)
    return


@app.cell
def _(display, env_2):
    terminated = False
    _obs, _state = env_2.reset()
    while not terminated:
        actions = env_2.action_space.sample(env_2.available_actions())
        _step = env_2.step(actions)
        # Equivalent to _step.is_terminal
        terminated = _step.done or _step.truncated
    display(env_2)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ## 4. Others
    ### 4.1 Setting the state
    You can force the state of the environment with `env.set_state(state)` where `state` is a `WorldState`.
    """)
    return


@app.cell
def _(Action, env, lle):
    _env = lle.level(6).build()
    _state1 = env.get_state()

    _env.step([Action.SOUTH.value] * 4)

    _state2 = _env.get_state()
    print(_state1, _state2)
    assert _state1 != _state2
    _env.set_state(_state1)
    return


if __name__ == "__main__":
    app.run()
