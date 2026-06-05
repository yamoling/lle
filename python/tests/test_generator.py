"""Public generator API tests."""

from multiprocessing import cpu_count

import lle
import pytest
from lle import CooperationLevel, World
from lle.generator._args import _GenerateArgs


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
        cooperation=("at-least", "cooperative"),
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
    args = _GenerateArgs("level6_style").resolve()
    assert args.height == 12
    assert args.width == 13
    assert args.n_agents == 4
    assert args.n_lasers == 3
    assert args.cooperation == ("exactly", CooperationLevel.MUTUAL)
    assert args.t_max == 21
    assert args.n_walls == (12 * 13) // 10


def test_random_defaults():
    args = _GenerateArgs("random").resolve()
    assert args.height == 5
    assert args.width == 5
    assert args.n_agents == 2
    assert args.n_lasers == 0
    assert args.cooperation is None
    assert args.n_jobs == 1


def test_n_jobs():
    args = _GenerateArgs("random").resolve(10)
    assert args.n_jobs == cpu_count() - 1

    args = _GenerateArgs("random", n_jobs="auto").resolve(10)
    assert args.n_jobs == cpu_count() - 1

    args = _GenerateArgs("random", n_jobs="auto").resolve(1)
    assert args.n_jobs == 1

    args = _GenerateArgs("random", n_jobs=5).resolve(10)
    assert args.n_jobs == 5


def test_t_min_defaults_to_zero():
    assert _GenerateArgs("random").resolve().t_min == 0
    assert _GenerateArgs("level6_style").resolve().t_min == 0


def test_t_min_passthrough():
    assert _GenerateArgs("random", t_min=4).resolve().t_min == 4


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
    args = _GenerateArgs("random", cooperation=True).resolve()
    assert args.cooperation == ("at-least", CooperationLevel.COOPERATIVE)

    args = _GenerateArgs("constructive", cooperation=True).resolve()
    assert args.cooperation == ("at-least", CooperationLevel.COOPERATIVE)

    args = _GenerateArgs("random", cooperation=False).resolve()
    assert args.cooperation == ("exactly", CooperationLevel.INDEPENDENT)

    args = _GenerateArgs("constructive", cooperation=False).resolve()
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
