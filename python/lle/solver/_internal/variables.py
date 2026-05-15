from pysat.formula import IDPool


class VariableFactory:
    def __init__(self):
        self.pool = IDPool(start_from=1)

    def agent(self, color, x, y, t):
        return self.pool.id(("agent", color, (x, y), t))

    def laser(self, color, x, y, t):
        return self.pool.id(("laser", color, (x, y), t))

    def beam(self, color, direction, x, y, t):
        return self.pool.id(("beam", color, direction, (x, y), t))

    def name(self, lit: int):
        return self.pool.obj(abs(lit))
