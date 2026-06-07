"""Public generator API tests."""

from multiprocessing import cpu_count

import lle
import pytest
from lle import CooperationLevel, World
from lle.generator._args import GenerateArgs


def test_generate_random_returns_world():
    world = lle.generate("random", n_agents=2, seed=0)
    assert isinstance(world, World)
    assert world.n_agents == 2


def test_generate_constructive_n_agents():
    w = lle.generate("constructive", n_agents=4)
    assert w.n_agents == 4


def test_generate_random_is_solvable():
    world = lle.generate("random", n_agents=2, seed=0)
    plan = lle.solve(world)
    assert plan is not None


def test_generate_random_cooperative_requires_cooperation():
    world = lle.generate("random", width=6, height=6, n_agents=2, n_lasers=2, cooperation=True, seed=0)
    assert lle.is_cooperative(world)


def test_generate_constructive_returns_world():
    world = lle.generate("constructive", height=6, width=6, n_agents=2, seed=0)
    assert isinstance(world, World)


def test_generate_constructive_cooperative():
    world = lle.generate("constructive", width=7, height=7, n_agents=2, n_lasers=2, cooperation=True, seed=0)
    assert lle.is_cooperative(world)


def test_generate_level6_style_always_cooperative():
    world = lle.generate(
        kind="level6_style",
        width=5,
        height=5,
        n_agents=4,
        n_lasers=3,
        seed=0,
        n=1,
        n_jobs=5,
        cooperation=("at-least", "asymmetric"),
        t_max=15,
    )
    assert world is not None
    assert lle.is_cooperative(world, t_max=15)


def test_generate_level6_style_rejects_cooperative_kwarg():
    # cooperative is not a parameter of level6_style; passing it must fail loudly.
    with pytest.raises(TypeError):
        lle.generate(kind="level6_style", size=(13, 13), agents=4, cooperative=False, seed=0)  # type: ignore


def test_generate_rejects_unknown_kind():
    with pytest.raises(ValueError):
        lle.generate(kind="not-a-real-kind", width=5, height=5)  # type: ignore[arg-type]


def test_generate_rejects_lasers_greater_than_agents():
    with pytest.raises(ValueError):
        lle.generate(kind="random", width=6, height=6, n_agents=2, n_lasers=5, seed=0)


def test_generate_cooperative_requires_at_least_two_agents():
    with pytest.raises(ValueError):
        lle.generate(kind="random", width=5, height=5, n_agents=1, cooperation=True, seed=0)


def test_generate_n():
    worlds = list(lle.generate("random", n=10))
    assert len(worlds) == 10

    w0 = worlds[0]
    for w in worlds:
        assert w.height == w0.height
        assert w.width == w0.width
        assert w.n_agents == w0.n_agents
        assert w.n_gems == w0.n_gems


def test_lvl6_defaults():
    args = GenerateArgs("level6_style").resolve()
    assert args.height == 12
    assert args.width == 13
    assert args.n_agents == 4
    assert args.n_lasers == 3
    assert args.cooperation == ("exactly", CooperationLevel.MUTUAL)
    assert args.t_max == 21
    assert args.n_walls == (12 * 13) // 10


def test_random_defaults():
    args = GenerateArgs("random").resolve()
    assert args.height == 5
    assert args.width == 5
    assert args.n_agents == 2
    assert args.n_lasers == 0
    assert args.cooperation is None
    assert args.n_jobs == 1


def test_n_jobs():
    args = GenerateArgs("random").resolve(10)
    assert args.n_jobs == cpu_count() - 1

    args = GenerateArgs("random", n_jobs="auto").resolve(10)
    assert args.n_jobs == cpu_count() - 1

    args = GenerateArgs("random", n_jobs="auto").resolve(1)
    assert args.n_jobs == 1

    args = GenerateArgs("random", n_jobs=5).resolve(10)
    assert args.n_jobs == 5


def test_t_min_defaults_to_zero():
    assert GenerateArgs("random").resolve().t_min == 0
    assert GenerateArgs("level6_style").resolve().t_min == 0


def test_t_min_passthrough():
    assert GenerateArgs("random", t_min=4).resolve().t_min == 4


def test_generate_rejects_t_min_greater_than_t_max():
    with pytest.raises(ValueError):
        lle.generate("random", width=5, height=5, t_max=5, t_min=10, seed=0)


def test_generate_rejects_negative_t_min():
    with pytest.raises(ValueError):
        lle.generate("random", width=5, height=5, t_min=-1, seed=0)


def test_generate_with_t_min_is_not_solvable_below_it():
    # t_min guarantees the level cannot be solved in fewer than t_min steps, while staying
    # solvable within t_max.
    t_min, t_max = 5, 12
    world = lle.generate("random", width=5, height=5, n_agents=2, t_min=t_min, t_max=t_max, seed=0)
    assert world is not None
    assert lle.solve(world, t_max=t_min - 1) is None  # not solvable below the lower bound
    assert lle.solve(world, t_max=t_max) is not None  # still solvable within the horizon


def test_default_cooperative():
    args = GenerateArgs("random", cooperation=True).resolve()
    assert args.cooperation == ("at-least", CooperationLevel.ASYMMETRIC)

    args = GenerateArgs("constructive", cooperation=True).resolve()
    assert args.cooperation == ("at-least", CooperationLevel.ASYMMETRIC)

    args = GenerateArgs("random", cooperation=False).resolve()
    assert args.cooperation == ("exactly", CooperationLevel.INDEPENDENT)

    args = GenerateArgs("constructive", cooperation=False).resolve()
    assert args.cooperation == ("exactly", CooperationLevel.INDEPENDENT)


def test_generator_produces_world_matching_requested_cooperation_profile():
    """Constructive generator with profile=ASYMMETRIC yields a world that classifies as ASYMMETRIC."""
    world = lle.generate(
        kind="constructive",
        width=6,
        height=6,
        n_agents=2,
        n_lasers=1,
        cooperation=CooperationLevel.ASYMMETRIC,
        t_max=15,
        seed=0,
    )
    assert world is not None
    assert lle.cooperation_level(world, t_max=15) is CooperationLevel.ASYMMETRIC


def test_classify_in_interval_respects_t_min():
    """classify_in_interval should not consider non-cooperative plans shorter than t_min."""
    from lle.solver.profile_analyzer import _classify_in_interval

    # Level 3 is asymmetric - cooperation is strictly required.
    level3 = lle.World.level(3)
    level = _classify_in_interval(level3, t_min=0, t_max=15)
    assert level is CooperationLevel.ASYMMETRIC

    # Level 1 is independent regardless of interval.
    level1 = lle.World.level(1)
    assert _classify_in_interval(level1, t_min=0, t_max=10) is CooperationLevel.INDEPENDENT


def test_generator_cooperation_holds_across_interval():
    """A world generated with cooperation and t_min should require cooperation across [t_min, t_max]."""
    from lle.solver.solver import solve_no_cooperation

    t_min, t_max = 3, 12
    world = lle.generate(
        kind="constructive",
        width=6,
        height=6,
        n_agents=2,
        n_lasers=1,
        cooperation=("at-least", CooperationLevel.ASYMMETRIC),
        t_min=t_min,
        t_max=t_max,
        seed=0,
    )
    assert world is not None
    # No non-cooperative solution should exist in [t_min, t_max].
    no_coop = solve_no_cooperation(world, t_min=t_min, t_max=t_max)
    assert no_coop is None, "a non-cooperative solution was found within [t_min, t_max]"
