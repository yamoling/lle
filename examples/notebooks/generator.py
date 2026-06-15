import marimo

__generated_with = "0.23.9"
app = marimo.App(width="medium")


@app.cell
def _():
    import marimo as mo

    return (mo,)


@app.cell(hide_code=True)
def _(mo):
    mo.md("""
    # Procedural world generation

    This notebook showcases some of the functionalities of the `lle.generator` submodule. This module provides procedural generation capabilities of world layouts under some constraints.

    Calling `lle.generate()` returns a `GeneratorBuilder` object that can be configured and then built to return one or multiple worlds.
    """)
    return


@app.cell
def _():
    from typing import Iterable, Sequence

    import lle
    import matplotlib.pyplot as plt

    def display(world: lle.World | Iterable[lle.World], *, titles: Sequence[str] | None = None):
        if isinstance(world, lle.World):
            plt.imshow(world.get_image())
            plt.axis("off")
        else:
            worlds = list(world)
            n_worlds = len(worlds)
            n_cols = min(n_worlds, 4)
            n_rows = max(1, n_worlds // 4)
            figsize = (5 * n_cols, 5 * n_rows)
            fig, axes = plt.subplots(n_rows, n_cols, figsize=figsize)
            if titles is None:
                titles = [f"World {i + 1}" for i in range(n_worlds)]
            for i, (w, title) in enumerate(zip(worlds, titles)):
                if n_rows == 1:
                    ax = axes[i]
                else:
                    ax = axes[i // n_cols, i % n_cols]
                ax.imshow(w.get_image())
                ax.set_title(title)
                ax.axis("off")
        plt.show()

    return display, lle, plt


@app.cell(hide_code=True)
def _(mo):
    mo.md("""
    ## Basic example
    You can simply call `lle.generate().build()` or `lle.generate().take(n)` to respectively generate one or `n` **solvable** worlds. The generator only outputs **solvable** worlds.
    """)
    return


@app.cell
def _(lle, plt):
    def simple_example():
        world = lle.generate().build()
        three_worlds = lle.generate().take(3)

        fig, axes = plt.subplots(1, 4, figsize=(20, 5))
        axes[0].imshow(world.get_image())
        axes[0].set_title("Single world")
        axes[0].axis("off")
        for i, w in enumerate(three_worlds):
            axes[i + 1].imshow(w.get_image())
            axes[i + 1].set_title(f"World {i + 1}")
            axes[i + 1].axis("off")
        plt.show()

    simple_example()
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ## Cooperative worlds
    Worlds can be constrained to be cooperative up to a specific number of time steps. There are multiple levels of cooperation.

    ### Mere cooperation
    The most simplistic constraint is to require cooperation, i.e. it is necessary (within `t_max` steps) for at least one agent to block a laser in order to reach the exits.
    """)
    return


@app.cell
def _(display, lle):
    cooperative_world = lle.generate(n_agents=2).cooperative(t_max=25).build()
    independent_world = lle.generate(n_agents=2).lasers(2).independent(t_max=25).build()
    display([cooperative_world, independent_world], titles=["Cooperative", "Independent"])
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Mutual cooperation
    Mutual cooperation can also be enforced, i.e. agent a helps agent b and vice-versa.
    """)
    return


@app.cell
def _(display, lle):
    mutual = lle.generate(n_agents=3).starts("edge").exits("opposite").lasers(2).mutual(t_max=30).build()
    display(mutual)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Chained cooperation
    Chained cooperation is a chain of at least two temporally sequential (or simultaneous) help events, i.e. help(a, b, t) and help(b, c, t+n) (with n $\geq$ 0).
    """)
    return


@app.cell
def _(display, lle):
    chained2 = lle.generate().lasers(2).chained().take(4)
    chained3 = lle.generate().lasers(3).chained().take(4)
    display([*chained2, *chained3])
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ###
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Interdependent cooperation
    Cooperation can also be constrained to be interdependent, i.e. there is a cycle in the dependencies. For instance, help(a, b, t), then help(b, c, t+1), then help(c, a, t+2).

    Note that for two agents, interdependence and mutual help are synonymous.
    """)
    return


@app.cell
def _(display, lle):
    inter2 = lle.generate(width=6, height=6, n_agents=2).interdependent().take(4)
    inter3 = lle.generate(width=7, height=7, n_agents=3).lasers(3, span=4).walls(8, style="shapes").interdependent(3).take(8)
    display([*inter2, *inter3])
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ## Wall specs
    How walls are layed out can also be configured. There are three main setups:
    - individually placed walls (default)
    - shaped-based, where predefined shaped of walls are selected (L-shaped, T-shaped, 2-line, 3-line, ...)
    - rooms, where the layout is split into multiple rooms with doors joining them
    """)
    return


@app.cell
def _(display, lle):
    individual_walls = lle.generate().walls(n=20, style="individual").build()
    shaped_walls = lle.generate().walls(n=20, style="shapes").build()
    rooms = lle.generate().rooms(n=4).lasers(1).build()
    display([individual_walls, shaped_walls, rooms], titles=["Individual walls", "Shapes walls", "4 Rooms"])
    return


if __name__ == "__main__":
    app.run()
