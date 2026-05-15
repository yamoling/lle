"""Public generator API tests."""

import lle
import pytest
from lle import World


def test_generate_random_returns_world():
    world = lle.generate("random", n_agents=2, seed=0)
    assert isinstance(world, World)
    assert world.n_agents == 2


def test_generate_random_is_solvable():
    world = lle.generate("random", n_agents=2, seed=0)
    plan = lle.solve(world)
    assert plan is not None


def test_generate_random_cooperative_requires_cooperation():
    world = lle.generate("random", width=6, height=6, n_agents=2, n_lasers=2, cooperative=True, seed=0, max_attempts=20_000)
    assert lle.is_cooperative(world)


def test_generate_constructive_returns_world():
    world = lle.generate("constructive", height=6, width=6, n_agents=2, seed=0)
    assert isinstance(world, World)


def test_generate_constructive_cooperative():
    world = lle.generate("constructive", width=7, height=7, n_agents=2, n_lasers=2, cooperative=True, seed=0)
    assert lle.is_cooperative(world)


def test_generate_level6_style_always_cooperative():
    world = lle.generate(kind="level6_style", width=13, height=13, n_agents=4, n_lasers=3, seed=0)
    assert lle.is_cooperative(world)


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
        lle.generate(kind="random", width=5, height=5, n_agents=1, cooperative=True, seed=0)
