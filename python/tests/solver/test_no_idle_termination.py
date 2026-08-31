"""A plan must not reach its objective before its last step.

Exit tiles are absorbing, so a state where *every* agent stands on an exit at some `t` before the
requested horizon can only be followed by all of them idling there until the horizon. Such a plan
is a shorter plan padded with `Stay`s, and the solver must not produce it.
"""

import lle
import pytest
from lle import World
from lle.solver import Solver
from lle.solver.clauses import ClauseGenerator
from pysat.solvers import Minisat22


def first_all_arrived_step(world: World, plan: list[tuple[lle.Action, ...]]) -> int | None:
    """Replay `plan` and return the first step index after which every agent has arrived.

    Steps are numbered from 1, so a plan that only completes at its horizon returns `len(plan)`.
    Returns `None` if the plan never brings every agent to an exit.
    """
    world.reset()
    for step, joint in enumerate(plan, start=1):
        world.step(joint)
        if all(agent.has_arrived for agent in world.agents):
            return step
    return None


def all_plans(world: World, path_length: int) -> list[list[tuple[lle.Action, ...]]]:
    """Enumerate the decoded plans of every model of the horizon-`path_length` formula."""
    generator = ClauseGenerator(world, path_length)
    clauses, assumptions = generator.generate(path_length, mode="standard", collect_gems=False)
    plans: list[list[tuple[lle.Action, ...]]] = []
    with Minisat22(bootstrap_with=clauses) as sat_solver:
        while sat_solver.solve(assumptions=assumptions):
            model = sat_solver.get_model()
            assert model is not None
            plans.append([tuple(row) for row in generator.decode_plan(model, path_length)])
            sat_solver.add_clause([-literal for literal in model])
    return plans


def test_no_model_completes_before_the_horizon():
    """Not one satisfying assignment may park every agent on an exit before the last step."""
    world = World("S0 . X")
    plans = all_plans(world, 4)
    assert plans, "the formula must remain satisfiable"
    early = [plan for plan in plans if first_all_arrived_step(world, plan) != len(plan)]
    assert early == [], f"these plans complete before their horizon: {early}"


@pytest.mark.parametrize("path_length", [3, 4, 5, 6])
def test_solved_plan_completes_exactly_at_its_horizon(path_length: int):
    world = World("S0 . . X")
    plan = lle.solve(world, 10, path_length=path_length)
    assert plan is not None
    assert first_all_arrived_step(world, plan) == path_length


def test_multi_agent_plan_completes_exactly_at_its_horizon():
    world = World("""
@  @ L1S @ X
S0 .  .  . .
S1 .  .  . .
@  @  .  @ X
""")
    plan = lle.solve(world, 12, path_length=10)
    assert plan is not None
    assert first_all_arrived_step(world, plan) == 10


def test_slack_horizons_stay_satisfiable():
    """Forbidding padded plans must not make a solvable world unsolvable at longer horizons."""
    world = World("S0 . . X")
    for path_length in range(3, 11):
        assert lle.solve(world, 10, path_length=path_length) is not None, (
            f"no plan of length {path_length}"
        )


def test_find_shortest_is_unaffected():
    plan = Solver(World("S0 . . X"), 10).find_shortest()
    assert plan is not None
    assert len(plan) == 3


def test_find_shortest_above_the_optimum_completes_at_its_horizon():
    """The incremental stream must enforce the rule too, not just the one-shot formula."""
    world = World("S0 . . X")
    plan = Solver(world, 10).find_shortest(t_min=6)
    assert plan is not None
    assert len(plan) == 6
    assert first_all_arrived_step(world, plan) == 6
