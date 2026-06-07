"""Base classes and shared context for SAT constraints.

Constraint classes use a shared context to avoid recomputing the same world
metadata, reachability sets, and SAT variable IDs.
"""

from abc import ABC, abstractmethod

from lle.types import Position

from ..variable_factory import VariableFactory
from .context import ConstraintContext


class ConstraintGenerator(ABC):
    def __init__(self, var: VariableFactory, ctx: ConstraintContext):
        self.ctx = ctx
        self.world = ctx.world
        self.var = var

    @abstractmethod
    def generate(self, t: int) -> list:
        """Generate the clauses for the given time step."""

    def _profile_method(self, _method_name: str, method_func):
        return list(method_func())

    def reachable_positions(self, t: int, *agents: int):
        return self.ctx.reachable_positions(t, *agents)

    def can_stay(self, t: int, pos: Position):
        """Check if staying in the same position for one more timestep is still compatible with reaching an exit."""
        return self.ctx.can_stay(t, pos)
