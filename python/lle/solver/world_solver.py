"""Internal SAT-based world solver. Public access via lle.solve / lle.is_cooperative."""

from __future__ import annotations

from enum import Enum

from pysat.solvers import Minisat22

from lle import Action, World

from ._constraints import (
    METHOD_LOCAL,
    ConstraintContext,
    InitializationConstraints,
    LaserConstraints,
    MovementConstraints,
    StrictLaserConstraints,
)
from ._constraints.base import Constraint
from ._internal import SATModel, VariableFactory


class LaserMode(Enum):
    STANDARD = "standard"
    STRICT = "strict"


class WorldSolver:
    """SAT-based solver for LLE worlds; verifies solvability within T_MAX steps."""

    def __init__(
        self,
        world: World,
        T_MAX: int = 10,
        *,
        laser_mode: LaserMode = LaserMode.STANDARD,
        movement_method: str = METHOD_LOCAL,
    ):
        self.world = world
        self.T_MAX = T_MAX
        self.var = VariableFactory()
        self.model = SATModel()
        self.movement_method = movement_method
        self.laser_mode = laser_mode

        self.ctx = ConstraintContext(world, self.var, T_MAX)
        self.constraints: list[Constraint] = [
            InitializationConstraints(self.ctx),
            MovementConstraints(self.ctx, movement_method=movement_method),
            self._build_laser_constraint(),
        ]
        self._model_built = False

    def _build_laser_constraint(self):
        if self.laser_mode is LaserMode.STANDARD:
            return LaserConstraints(self.ctx)
        if self.laser_mode is LaserMode.STRICT:
            return StrictLaserConstraints(self.ctx)
        raise ValueError(f"Unknown laser_mode: {self.laser_mode}")

    def build_model(self):
        if self._model_built:
            return
        for constraint in self.constraints:
            self.model.extend(constraint.generate())
        self._model_built = True

    def solve(self):
        self.build_model()
        with Minisat22(bootstrap_with=self.model.cnf.clauses) as solver:
            result = solver.solve()
            model = solver.get_model() if result else None
        return result, model

    def extract_plan(self, model) -> list[tuple[Action, ...]]:
        positions: dict[int, dict[int, tuple[int, int]]] = {}
        for lit in model:
            if lit <= 0:
                continue
            obj = self.var.pool.obj(abs(lit))
            if not obj or obj[0] != "agent":
                continue
            _, color, (x, y), t = obj
            positions.setdefault(color, {})[t] = (x, y)

        agent_colors = sorted(positions.keys())
        plan: list[tuple[Action, ...]] = []
        for t in range(self.T_MAX):
            row: list[Action] = []
            for color in agent_colors:
                x1, y1 = positions[color][t]
                x2, y2 = positions[color][t + 1]
                dx, dy = x2 - x1, y2 - y1
                if dx == 0 and dy == 0:
                    a = Action.STAY
                elif dx == -1 and dy == 0:
                    a = Action.NORTH
                elif dx == 1 and dy == 0:
                    a = Action.SOUTH
                elif dx == 0 and dy == -1:
                    a = Action.WEST
                elif dx == 0 and dy == 1:
                    a = Action.EAST
                else:
                    raise ValueError(f"Invalid movement for agent {color} at t={t}->{t + 1}")
                row.append(a)
            plan.append(tuple(row))
        return plan
