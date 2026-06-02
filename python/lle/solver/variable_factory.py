from pysat.formula import IDPool


class VariableFactory:
    """Variable factory for SAT encodings used by the solver."""

    def __init__(self):
        self.pool = IDPool(start_from=1)

    def agent(self, color: int, x: int, y: int, t: int) -> int:
        return self.pool.id(("agent", color, (x, y), t))

    def agent_at_exit(self, color: int, t: int) -> int:
        return self.pool.id(("agent_at_exit", color, t))

    def done(self, t: int) -> int:
        return self.pool.id(("done", t))

    def laser(self, colour: int, direction: tuple[int, int], x: int, y: int, t: int) -> int:
        """
        Variable that determines whether the laser at position (x, y) of the given colour
        and direction is active at time step t.
        """
        return self.pool.id(("laser", direction, colour, (x, y), t))

    # def beam(self, color: int, direction: tuple[int, int], source: tuple[int, int], x: int, y: int, t: int) -> int:
    #     """Get the variable that determines whether beam is active at (x, y) at time t."""
    #     return self.pool.id(("beam", color, direction, source, (x, y), t))

    def name(self, lit: int):
        return self.pool.obj(abs(lit))
