"""Tests for the CooperationConstraints class and solve_no_cooperation.

Covers:
- no_blocking_clauses: produces UNSAT on cooperative worlds, allows SAT on independent ones
- laser_blocked variable: correctly set when the same-colour agent blocks
- coop_event variable: correctly set when a helper is upstream of a beneficiary
- depends_on variable: correctly aggregated over the full horizon
- require_asymmetric / require_mutual / require_fully_coupled: level enforcement
"""

from __future__ import annotations

import lle
import pytest
from lle import Action, World
from lle.solver.constraints_old import (
    ConstraintContext,
    CooperationConstraints,
    InitializationConstraints,
    LaserConstraints,
    MovementConstraints,
    ObjectiveGenerator,
)
from lle.solver.variable_factory import VariableFactory
from pysat.solvers import Minisat22


def extract_plan(var: VariableFactory, model: list[int], t_end: int) -> list[tuple[Action, ...]]:
    """Decode a SAT model produced by the (Python) `constraints_old` pipeline into a joint-action plan."""
    positions = dict[int, dict[int, tuple[int, int]]]()
    for lit in model:
        if lit <= 0:
            continue
        obj = var.pool.obj(lit)
        if not obj:
            continue
        if obj[0] == "agent":
            _, color, x, y, t = obj
            positions.setdefault(color, {})[t] = (x, y)
    agent_colors = sorted(positions.keys())
    plan: list[tuple[Action, ...]] = []
    for t in range(t_end):
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


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
def _solve_with_coop(
    world: World, t_max: int = 15
) -> tuple[list | None, list[int] | None, VariableFactory, ConstraintContext, CooperationConstraints]:
    """Solve with cooperation tracking; return (plan, model, var, ctx, coop)."""
    ctx = ConstraintContext(world, t_max)
    t_min = max(ctx.solution_lower_bound, 0)
    var = VariableFactory()
    coop = CooperationConstraints(var, ctx)
    generators = [
        InitializationConstraints(var, ctx),
        MovementConstraints(var, ctx),
        LaserConstraints(var, ctx),
        coop,
    ]
    objective = ObjectiveGenerator(var, ctx)

    clauses = [c for gen in generators for t in range(t_min) for c in gen.generate(t)]
    for t in range(t_min, t_max + 1):
        clauses.extend(c for gen in generators for c in gen.generate(t))
        finalize = list(coop.finalize_depends_on(t))
        with Minisat22(bootstrap_with=clauses + finalize) as solver:
            solver.append_formula(objective.generate(t))
            if solver.solve():
                model = solver.get_model()
                assert model is not None
                return extract_plan(var, model, t), model, var, ctx, coop
    return None, None, var, ctx, coop


def _solve_with_level_constraint(
    world: World,
    level_method: str,
    t_max: int = 15,
) -> tuple[bool, list[int] | None, VariableFactory, CooperationConstraints]:
    """Solve adding a cooperation-level constraint; return (sat, model, var, coop)."""
    ctx = ConstraintContext(world, t_max)
    t_min = max(ctx.solution_lower_bound, 0)
    var = VariableFactory()
    coop = CooperationConstraints(var, ctx)
    generators = [
        InitializationConstraints(var, ctx),
        MovementConstraints(var, ctx),
        LaserConstraints(var, ctx),
        coop,
    ]
    objective = ObjectiveGenerator(var, ctx)

    clauses = [c for gen in generators for t in range(t_min) for c in gen.generate(t)]
    for t in range(t_min, t_max + 1):
        clauses.extend(c for gen in generators for c in gen.generate(t))
        finalize = list(coop.finalize_depends_on(t))
        level_clauses = list(getattr(coop, level_method)())
        with Minisat22(bootstrap_with=clauses + finalize + level_clauses) as solver:
            solver.append_formula(objective.generate(t))
            if solver.solve():
                model = solver.get_model()
                assert model is not None
                return True, model, var, coop
    return False, None, var, coop


# ---------------------------------------------------------------------------
# no_blocking_clauses
# ---------------------------------------------------------------------------


def test_all_clauses_are_unit_and_negated():
    ctx = ConstraintContext(World.level(3), 10)
    var = VariableFactory()
    coop = CooperationConstraints(var, ctx)
    for t in range(10 + 1):
        for clause in coop.no_blocking_clauses(t):
            assert len(clause) == 1, "each no-blocking clause must be a unit clause"
            assert clause[0] < 0, "the literal must be a negated agent variable"


@pytest.mark.parametrize("level", [1, 2])
def test_solvable_independently(level: int):
    assert lle.solve(World.level(level), 10, mode="no-cooperation") is not None


@pytest.mark.parametrize("level", [3, 4, 5, 6])
def test_level_3_becomes_unsolvable(level: int):
    assert lle.solve(World.level(level), 21, mode="no-cooperation") is None


def test_laser_blocked_references_blockable_positions_only():
    """laser_blocked variables must only be created for sources with reachable beam tiles."""
    ctx = ConstraintContext(World.level(3), 10)
    var = VariableFactory()
    coop = CooperationConstraints(var, ctx)
    for t in range(11):
        for _ in coop._laser_blocked_definitions(t):
            pass  # just run to populate var pool

    for lit_id in range(1, var.pool.top + 1):
        obj = var.key(lit_id)
        if obj is None or obj[0] != "laser_blocked":
            continue
        _, laser_id, t = obj
        source = next(s for s in ctx.laser_paths if s.laser_id == laser_id)
        assert bool(ctx.reachable_laser_paths[source][t]), f"laser_blocked({laser_id}, {t}) created with no blockable positions"


# ---------------------------------------------------------------------------
# coop_event variable
# ---------------------------------------------------------------------------
def test_coop_event_true_in_cooperative_plan():
    plan, model, var, ctx, coop = _solve_with_coop(World.level(3), t_max=10)
    assert plan is not None and model is not None

    coop_true = any(lit > 0 for lit in model if (obj := var.key(lit)) is not None and obj[0] == "coop_event")
    assert coop_true


def test_no_coop_event_in_independent_world():
    plan, model, var, ctx, coop = _solve_with_coop(World("S0 . X"), t_max=5)
    assert plan is not None and model is not None

    coop_true = any(lit > 0 for lit in model if (obj := var.key(lit)) is not None and obj[0] == "coop_event")
    assert not coop_true


def test_coop_term_helper_is_always_upstream():
    """By construction, blocker_idx < benef_idx for every coop_term variable."""
    plan, model, var, ctx, coop = _solve_with_coop(World.level(3), t_max=10)
    assert plan is not None and model is not None

    for lit in model:
        if lit <= 0:
            continue
        obj = var.key(lit)
        if obj is None or obj[0] != "coop_term":
            continue
        # ("coop_term", helper, beneficiary, laser_id, blocker_idx, benef_idx, t)
        _, _helper, _bene, _lid, blocker_idx, benef_idx, _t = obj
        assert blocker_idx < benef_idx


def test_coop_event_implies_laser_blocked():
    """If coop_event is true for (h, b, laser_id, t), laser_blocked(laser_id, t) must be true."""
    plan, model, var, ctx, coop = _solve_with_coop(World.level(3), t_max=10)
    assert plan is not None and model is not None

    true_lits = {lit for lit in model if lit > 0}
    for lit in model:
        if lit <= 0:
            continue
        obj = var.key(lit)
        if obj is None or obj[0] != "coop_event":
            continue
        _, _helper, _bene, laser_id, t = obj
        blocked_id = var.pool.id(("laser_blocked", laser_id, t))
        assert blocked_id in true_lits, "coop_event true but laser_blocked not set"


# ---------------------------------------------------------------------------
# depends_on variable
# ---------------------------------------------------------------------------


class TestDependsOn:
    def test_depends_on_set_after_cooperation(self):
        plan, model, var, ctx, coop = _solve_with_coop(World.level(3), t_max=10)
        assert plan is not None and model is not None
        dep_edges = coop.extract_dependency_edges(model)
        assert len(dep_edges) > 0

    def test_depends_on_empty_in_independent_world(self):
        plan, model, var, ctx, coop = _solve_with_coop(World("S0 . X"), t_max=5)
        assert plan is not None and model is not None
        assert len(coop.extract_dependency_edges(model)) == 0

    def test_depends_on_level_6_has_bidirectional_edges(self):
        plan, model, var, ctx, coop = _solve_with_coop(World.level(6), t_max=21)
        assert plan is not None and model is not None
        dep_edges = coop.extract_dependency_edges(model)
        helpers = {h for h, _ in dep_edges}
        beneficiaries = {b for _, b in dep_edges}
        assert helpers & beneficiaries, "level 6 (mutual) must have agents appearing on both sides"

    def test_depends_on_level_3_is_one_directional(self):
        plan, model, var, ctx, coop = _solve_with_coop(World.level(3), t_max=10)
        assert plan is not None and model is not None
        dep_edges = coop.extract_dependency_edges(model)
        # Level 3 is asymmetric: at most one direction
        for h, b in dep_edges:
            assert (b, h) not in dep_edges, f"level 3 should not have bidirectional edge {h}↔{b}"


# ---------------------------------------------------------------------------
# require_asymmetric
# ---------------------------------------------------------------------------
def test_sat_on_cooperative_world():
    sat, model, var, coop = _solve_with_level_constraint(World.level(3), "require_asymmetric", t_max=10)
    assert sat
    assert model is not None
    dep_edges = coop.extract_dependency_edges(model)
    assert len(dep_edges) >= 1


def test_unsat_on_independent_world():
    sat, _model, _var, _coop = _solve_with_level_constraint(World("S0 . X"), "require_asymmetric", t_max=5)
    assert not sat


def test_unsat_on_level_1():
    sat, _, _, _ = _solve_with_level_constraint(World.level(1), "require_asymmetric", t_max=10)
    assert not sat


# ---------------------------------------------------------------------------
# require_mutual
# ---------------------------------------------------------------------------
class TestRequireMutual:
    def test_sat_on_level_6(self):
        sat, model, var, coop = _solve_with_level_constraint(World.level(6), "require_mutual", t_max=21)
        assert sat
        assert model is not None
        dep_edges = coop.extract_dependency_edges(model)
        helpers = {h for h, _ in dep_edges}
        beneficiaries = {b for _, b in dep_edges}
        assert helpers & beneficiaries, "mutual plan must have at least one bidirectional pair"

    def test_unsat_on_level_3(self):
        """Level 3 is strictly asymmetric – require_mutual must be UNSAT."""
        sat, _, _, _ = _solve_with_level_constraint(World.level(3), "require_mutual", t_max=10)
        assert not sat

    def test_unsat_on_independent_world(self):
        sat, _, _, _ = _solve_with_level_constraint(World("S0 . X"), "require_mutual", t_max=5)
        assert not sat


# ---------------------------------------------------------------------------
# require_fully_coupled
# ---------------------------------------------------------------------------


class TestRequireFullyCoupled:
    def test_sat_on_level_4(self):
        world = World.level(4)
        sat, model, var, coop = _solve_with_level_constraint(world, "require_fully_coupled", t_max=10)
        assert sat
        assert model is not None
        dep_edges = coop.extract_dependency_edges(model)
        n = world.n_agents
        for a in range(n):
            for b in range(n):
                if a != b:
                    assert (a, b) in dep_edges, f"fully-coupled plan missing edge {a}→{b}"

    def test_unsat_on_level_3(self):
        sat, _, _, _ = _solve_with_level_constraint(World.level(3), "require_fully_coupled", t_max=10)
        assert not sat

    def test_unsat_on_independent_world(self):
        sat, _, _, _ = _solve_with_level_constraint(World("S0 . X"), "require_fully_coupled", t_max=5)
        assert not sat


# ---------------------------------------------------------------------------
# Integration: solve_no_cooperation vs. lle.is_cooperative
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "level,t_max,expect_coop",
    [
        (1, 10, False),
        (2, 10, False),
        (3, 10, True),
        (4, 10, True),
        (6, 21, True),
    ],
)
def test_solve_no_cooperation_agrees_with_is_cooperative(level: int, t_max: int, expect_coop: bool):
    """solve_no_cooperation must agree with lle.is_cooperative for all canonical levels."""
    import lle

    world = World.level(level)
    no_coop = lle.solve(world, t_max, mode="no-cooperation")
    is_coop = lle.is_cooperative(world, t_max=t_max)
    assert (no_coop is None) == is_coop == expect_coop
