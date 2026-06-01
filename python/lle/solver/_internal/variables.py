class VariableFactory:
    """Variable factory for SAT encodings used by the solver."""

    def __init__(self):
        from pysat.formula import IDPool  # pyright: ignore[reportMissingImports]

        self.pool = IDPool(start_from=1)

    def agent(self, color: int, x: int, y: int, t: int):
        return self.pool.id(("agent", color, (x, y), t))

    def agent_at_exit(self, color: int, t: int):
        return self.pool.id(("agent_at_exit", color, t))

    def done(self, t: int):
        return self.pool.id(("done", t))

    def laser(self, color: int, x: int, y: int, t: int):
        return self.pool.id(("laser", color, (x, y), t))

    def beam(self, color: int, direction: tuple[int, int], source: tuple[int, int], x: int, y: int, t: int):
        return self.pool.id(("beam", color, direction, source, (x, y), t))

    def name(self, lit: int):
        return self.pool.obj(abs(lit))
