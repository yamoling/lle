"""Tests of the `CLLE` builder (round A, §4): `clle.level`, `clle.from_str` and `clle.from_file`,
the physics setters, the observation types, the death strategy and the default names."""

from typing import Any

import lle
import lle.continuous as clle
import pytest
from lle.continuous import ContinuousWorld, ObservationType, PhysicsParams

from .common import CORRIDOR, EAST, SAME_COLOUR, TWO_AGENTS


def test_defaults():
    env = clle.from_str(CORRIDOR).build()
    assert env.world.params == PhysicsParams()
    assert env.name == "CLLE"
    assert env.reward_space.shape == (1,)
    assert env.observation_shape == (ContinuousWorld(CORRIDOR).observation_size,)
    env.reset()
    # The default action_repeat is 10.
    assert env.step([EAST]).info["n_ticks"] == 10


def test_level_from_str_and_from_file_load_the_same_map(tmp_path):
    level_6 = lle.World.level(6).world_string
    path = tmp_path / "lvl6"
    path.write_text(level_6)
    for builder in (clle.level(6), clle.from_file(str(path)), clle.from_str(level_6)):
        env = builder.build()
        assert env.n_agents == 4
        assert env.world.get_state() == ContinuousWorld.level(6).get_state()


def test_from_file_reads_a_toml_map(tmp_path):
    path = tmp_path / "same_colour.toml"
    path.write_text(SAME_COLOUR)
    env = clle.from_file(str(path)).build()
    assert list(env.world.agents_colours) == [0, 0]


def test_physics_setters_reach_the_world_params():
    env = (
        clle.from_str(CORRIDOR)
        .dt(0.04)
        .agent_radius(0.25)
        .max_speed(1.5)
        .action_model("acceleration")
        .max_acceleration(5.0)
        .damping(0.8)
        .gem_radius(0.2)
        .beam_width(0.05)
        .build()
    )
    assert env.world.params == PhysicsParams(
        dt=0.04,
        agent_radius=0.25,
        max_speed=1.5,
        action_model="acceleration",
        max_acceleration=5.0,
        damping=0.8,
        gem_radius=0.2,
        beam_width=0.05,
    )


def test_action_repeat_setter():
    env = clle.from_str(CORRIDOR).action_repeat(3).build()
    env.reset()
    step = env.step([EAST])
    assert step.info["n_ticks"] == 3
    assert env.world.agents_positions[0][0] == pytest.approx(0.8)


@pytest.mark.parametrize(
    "setter, value",
    [
        ("dt", 0.0),
        ("agent_radius", 0.5),
        ("max_speed", 10.0),  # max_speed * dt = 0.5 > agent_radius / 2
        ("damping", 1.5),
        ("gem_radius", -0.1),
        ("beam_width", 0.0),
        ("action_model", "unicycle"),
    ],
)
def test_invalid_params_raise_at_build(setter: str, value: Any):
    builder = getattr(clle.from_str(CORRIDOR), setter)(value)
    with pytest.raises(ValueError):
        builder.build()


def test_default_names(tmp_path):
    path = tmp_path / "corridor"
    path.write_text(CORRIDOR)
    assert clle.level(3).build().name == "CLLE-lvl3"
    assert clle.from_file(str(path)).build().name == "CLLE-corridor"
    assert clle.from_str(CORRIDOR).build().name == "CLLE"
    assert clle.from_str(CORRIDOR).name("my-env").build().name == "my-env"


@pytest.mark.parametrize("value", ["state", ObservationType.STATE])
def test_obs_and_state_type_accept_state(value: Any):
    env = clle.from_str(TWO_AGENTS).obs_type(value).state_type(value).build()
    world = ContinuousWorld(TWO_AGENTS)
    assert env.observation_shape == (world.observation_size,)
    assert env.state_shape == (world.state_size,)


def test_end_death_strategy_is_accepted():
    env = clle.from_str(CORRIDOR).death_strategy("end").build()
    env.reset()
    assert not env.step([EAST]).done


def test_respawn_death_strategy_is_not_implemented():
    with pytest.raises(NotImplementedError):
        clle.from_str(CORRIDOR).death_strategy("respawn").build()
