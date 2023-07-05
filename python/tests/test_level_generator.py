import pytest
import lle


def test_level_generator():
    pass
    # env = lle.DynamicLaserEnv(width=5, height=5, num_agents=4, num_gems=5, num_lasers=1)

    # env.reset()
    # world = env.world
    # assert env.n_agents == 4
    # assert len(world._gems) == 5
    # # The amount of lasers could be lower than the requested amount in case a laser kills an agent on its starting position
    # assert len(world._laser_sources_pos) <= 1


def test_impossible_level_generator():
    pass
    # with pytest.raises(ValueError):
    #     # Too many gems
    #     lle.DynamicLaserEnv(width=5, height=5, num_gems=50)

    # with pytest.raises(AssertionError):
    #     # Too many agents
    #     lle.DynamicLaserEnv(width=5, height=5, num_agents=5)
