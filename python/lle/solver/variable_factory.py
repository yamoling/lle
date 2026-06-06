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
        """Variable that determines whether the laser at position (i, j) of the given ID is active at time step t."""
        return self.pool.id(("laser", laser_id, i, j, t))

    # ------------------------------------------------------------------
    # Cooperation-tracking variables
    # ------------------------------------------------------------------

    def laser_blocked(self, laser_id: int, t: int) -> int:
        """True iff the same-colour agent stands on laser ``laser_id``'s beam at time ``t``."""
        return self.pool.id(("laser_blocked", laser_id, t))

    def coop_term(self, helper: int, beneficiary: int, laser_id: int, blocker_idx: int, benef_idx: int, t: int) -> int:
        """Auxiliary variable: helper is at beam[blocker_idx] AND beneficiary is at beam[benef_idx] at time t.

        Used to build the OR-definition of ``coop_event``.
        blocker_idx < benef_idx is required (helper is upstream of beneficiary).
        """
        return self.pool.id(("coop_term", helper, beneficiary, laser_id, blocker_idx, benef_idx, t))

    def coop_event(self, helper: int, beneficiary: int, laser_id: int, t: int) -> int:
        """True iff helper blocks laser ``laser_id`` while beneficiary is at a downstream beam position at time t."""
        return self.pool.id(("coop_event", helper, beneficiary, laser_id, t))

    def depends_on(self, beneficiary: int, helper: int) -> int:
        """True iff helper ever helps beneficiary during the plan (across all lasers and timesteps)."""
        return self.pool.id(("depends_on", beneficiary, helper))

    def mutual(self, a: int, b: int) -> int:
        """True iff agents a and b mutually depend on each other.  Canonical form: min < max."""
        lo, hi = (a, b) if a < b else (b, a)
        return self.pool.id(("mutual", lo, hi))

    # ------------------------------------------------------------------

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

    @overload
    def exists(self, kind: Literal["laser_blocked"], laser_id: int, t: int, /) -> bool: ...

    @overload
    def exists(self, kind: Literal["coop_event"], helper: int, beneficiary: int, laser_id: int, t: int, /) -> bool: ...

    @overload
    def exists(self, kind: Literal["depends_on"], beneficiary: int, helper: int, /) -> bool: ...

    def exists(self, *args) -> bool:
        return args in self.pool.obj2id
