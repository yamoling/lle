from typing import Literal, overload

from pysat.formula import Atom, IDPool


class VariableFactory:
    """Variable factory for SAT encodings used by the solver."""

    def __init__(self):
        self.pool = IDPool(start_from=1)

    @overload
    def agent(self, colour: int, x: int, y: int, t: int) -> int: ...
    @overload
    def agent(self, colour: int, x: int, y: int, t: int, *, atom: Literal[True]) -> Atom: ...

    def agent(self, colour: int, x: int, y: int, t: int, *, atom: bool = False):
        var = self.pool.id(("agent", colour, x, y, t))
        if atom:
            return Atom(var)
        return var

    def agent_at_exit(self, colour: int, t: int) -> int:
        return self.pool.id(("agent_at_exit", colour, t))

    def done(self, t: int) -> int:
        return self.pool.id(("done", t))

    @overload
    def laser(self, laser_id: int, x: int, y: int, t: int) -> int: ...

    @overload
    def laser(self, laser_id: int, x: int, y: int, t: int, *, atom: Literal[True]) -> Atom: ...

    def laser(self, laser_id: int, x: int, y: int, t: int, *, atom: bool = False):
        """
        Variable that determines whether the laser at position (x, y) of the given ID is active at time step t.
        """
        var = self.pool.id(("laser", laser_id, x, y, t))
        if atom:
            return Atom(var)
        return var

    def name(self, lit: int) -> tuple | None:
        return self.pool.obj(abs(lit))

    @overload
    def exists(self, kind: Literal["laser"], laser_id: int, x: int, y: int, t: int, /) -> bool: ...

    @overload
    def exists(self, kind: Literal["agent"], colour: int, x: int, y: int, t: int, /) -> bool: ...

    @overload
    def exists(self, kind: Literal["agent_at_exit"], colour: int, t: int, /) -> bool: ...

    @overload
    def exists(self, kind: Literal["done"], t: int, /) -> bool: ...

    def exists(self, *args) -> bool:
        return args in self.pool.obj2id
