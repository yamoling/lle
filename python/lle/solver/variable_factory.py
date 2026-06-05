from typing import Literal, overload

from pysat.formula import IDPool


class VariableFactory:
    """Variable factory for SAT encodings used by the solver."""

    def __init__(self):
        self.pool = IDPool(start_from=1)

    def agent(self, agent_num: int, i: int, j: int, t: int) -> int:
        return self.pool.id(("agent", agent_num, i, j, t))

    def agent_at_exit(self, colour: int, t: int) -> int:
        return self.pool.id(("agent_at_exit", colour, t))

    def done(self, t: int) -> int:
        return self.pool.id(("done", t))

    def laser(self, laser_id: int, i: int, j: int, t: int) -> int:
        """
        Variable that determines whether the laser at position (i, j) of the given ID is active at time step t.
        """
        return self.pool.id(("laser", laser_id, i, j, t))

    def name(self, lit: int) -> str:
        key = self.pool.obj(abs(lit))
        if key is None:
            return ""
        return " ".join((str(k) for k in key))

    def key(self, lit: int) -> tuple | None:
        return self.pool.obj(abs(lit))

    @overload
    def exists(self, kind: Literal["laser"], laser_id: int, i: int, j: int, t: int, /) -> bool: ...

    @overload
    def exists(self, kind: Literal["agent"], colour: int, i: int, j: int, t: int, /) -> bool: ...

    @overload
    def exists(self, kind: Literal["agent_at_exit"], colour: int, t: int, /) -> bool: ...

    @overload
    def exists(self, kind: Literal["done"], t: int, /) -> bool: ...

    def exists(self, *args) -> bool:
        return args in self.pool.obj2id
