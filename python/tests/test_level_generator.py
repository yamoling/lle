import pytest
import laser_env


def test_level_generator():
    env = laser_env.DynamicLaserEnv(width=5, height=5, num_agents=4, num_gems=5, num_lasers=1)

    env.reset()
    world = env.world
    assert env.n_agents == 4
    assert len(world._gems) == 5
    # The amount of lasers could be lower than the requested amount in case a laser kills an agent on its starting position
    assert len(world._laser_sources_pos) <= 1


def test_impossible_level_generator():
    with pytest.raises(ValueError):
        # Too many gems
        laser_env.DynamicLaserEnv(width=5, height=5, num_gems=50)

    with pytest.raises(AssertionError):
        # Too many agents
        laser_env.DynamicLaserEnv(width=5, height=5, num_agents=5)
