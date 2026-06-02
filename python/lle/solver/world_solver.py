from __future__ import annotations

from ..world import Action, World
from ._constraints import (
    METHOD_LOCAL,
    ConstraintContext,
    InitializationConstraints,
    LaserConstraints,
    MovementConstraints,
    StrictLaserConstraints,
)
from ._constraints.base import ConstraintGenerator
from ._internal import SATModel
from .laser_mode import LaserMode
from .variable_factory import VariableFactory


class WorldSolver:
    """SAT-based solver for `World` objects.

    Public code should call `lle.solve`, `lle.is_cooperative`, or
    `lle.cooperation_level` instead of instantiating this class directly.
    """

    def __init__(self, world: World, t_max: int = 10, *, laser_mode: LaserMode = LaserMode.STANDARD, movement_method: str = METHOD_LOCAL):
        self.world = world
        self.t_max = t_max
        self.var = VariableFactory()
        self.model = SATModel()
        self.movement_method = movement_method
        self.laser_mode = laser_mode

        self.ctx = ConstraintContext(world, 0, t_max)
        self.constraints: list[ConstraintGenerator] = [
            InitializationConstraints(self.var, self.ctx),
            MovementConstraints(self.var, self.ctx),
            laser_mode.get(self.var, self.ctx),
        ]
        self._model_built = False

    def _build_laser_constraint(self):
        if self.laser_mode is LaserMode.STANDARD:
            return LaserConstraints(self.var, self.ctx)
        if self.laser_mode is LaserMode.STRICT:
            return StrictLaserConstraints(self.var, self.ctx)
        raise ValueError(f"Unknown laser_mode: {self.laser_mode}")

    def build_model(self):
        if self._model_built:
            return
        for constraint in self.constraints:
            for t in range(self.t_max):
                self.model.extend(constraint.generate(t))
        self._model_built = True

    def solve(self):
        from pysat.solvers import Minisat22

        self.build_model()
        with Minisat22(bootstrap_with=self.model.cnf.clauses) as solver:
            result = solver.solve()
            model = solver.get_model() if result else None
        return result, model

    def solve_sat(self):
        """Alias for the default satisfiability solve.

        This keeps the original fixed-horizon SAT behavior available even when
        `solve_shortest` is used for optimization.
        """

        return self.solve()

    def solve_shortest(self):
        """Find the shortest plan within the current `t_max` horizon.

        The hard constraints are shared with the SAT solve. The only extra
        encoding is a small MaxSAT objective that rewards earlier completion.
        """
        from pysat.examples.rc2 import RC2
        from pysat.formula import WCNF

        self.build_model()
        done_vars, completion_clauses = self._completion_clauses()
        wcnf = WCNF()
        for clause in self.model.cnf.clauses:
            wcnf.append(clause)
        for clause in completion_clauses:
            wcnf.append(clause)
        for done_t in done_vars:
            wcnf.append([done_t], weight=1)
        wcnf.append(done_vars)
        with RC2(wcnf) as solver:
            model = solver.compute()
        return model is not None, model

    def solve_hybrid(self) -> tuple[bool, list | None]:
        """Find the shortest plan using incremental SAT with clause reuse.

        This keeps a single SAT instance alive and tries the horizon assumptions
        in increasing order, so learned clauses are reused across checks.
        """
        from pysat.solvers import Minisat22

        self.build_model()
        done_vars, completion_clauses = self._completion_clauses()
        bootstrap = [*self.model.cnf.clauses, *completion_clauses]
        with Minisat22(bootstrap_with=bootstrap) as solver:
            for done_t in done_vars:
                if solver.solve(assumptions=[done_t]):
                    model = solver.get_model()
                    return True, model
        return False, None

    def _completion_clauses(self):
        """Return the completion variables and the corresponding hard clauses."""
        agent_var = self.ctx.agent_var
        exit_positions = tuple(self.ctx.exits)
        agent_colors = [agent.color for agent, _ in self.ctx.agents]

        done_vars = []
        completion_clauses = []
        for t in range(self.t_max + 1):
            done_t = self.var.done(t)
            done_vars.append(done_t)
            at_exit_vars = []
            for color in agent_colors:
                at_exit = self.var.agent_at_exit(color, t)
                at_exit_vars.append(at_exit)
                exit_lits = [agent_var[color, x, y, t] for x, y in exit_positions if (color, x, y, t) in agent_var]
                if not exit_lits:
                    # No exit position is available for this agent at this time,
                    # so this branch cannot witness a valid solution.
                    completion_clauses.append([-at_exit])
                else:
                    # at_exit -> some exit position
                    completion_clauses.append([-at_exit, *exit_lits])
                    # any exit position -> at_exit
                    for exit_lit in exit_lits:
                        completion_clauses.append([-exit_lit, at_exit])
                # done_t -> this agent is at an exit
                completion_clauses.append([-done_t, at_exit])
            if at_exit_vars:
                # all agents at an exit -> done_t
                completion_clauses.append([*[-lit for lit in at_exit_vars], done_t])
        return done_vars, completion_clauses

    def extract_plan(self, model, horizon: int | None = None) -> list[tuple[Action, ...]]:
        positions = dict[int, dict[int, tuple[int, int]]]()
        done_times: list[int] = []
        for lit in model:
            if lit <= 0:
                continue
            obj = self.var.pool.obj(abs(lit))
            if not obj:
                continue
            if obj[0] == "agent":
                _, color, (x, y), t = obj
                positions.setdefault(color, {})[t] = (x, y)
            elif obj[0] == "done":
                _, t = obj
                done_times.append(t)
        agent_colors = sorted(positions.keys())
        if horizon is None:
            horizon = min(done_times) if done_times else self.t_max
        plan: list[tuple[Action, ...]] = []
        for t in range(horizon):
            row: list[Action] = []
            for color in agent_colors:
                y1, x1 = positions[color][t]
                y2, x2 = positions[color][t + 1]
                dx, dy = x2 - x1, y2 - y1
                try:
                    a = Action.from_delta(dx, dy)
                except ValueError as e:
                    raise ValueError(f"Invalid movement for agent {color} at t={t}->{t + 1}") from e
                row.append(a)
            plan.append(tuple(row))
        return plan
