import numpy as np
import rlenv
import laser_env



def test_save_summary():
    env = laser_env.DynamicLaserEnv()
    summary = env.summary()
    env2 = laser_env.DynamicLaserEnv.from_summary(summary)
    assert env2.n_actions == env.n_actions
    assert env2.n_agents == env.n_agents
    assert env2.observation_shape == env.observation_shape
    assert env2.state_shape == env.state_shape
    assert env2.extra_feature_shape == env.extra_feature_shape

def test_restore_static_from_dynamic():
    env = laser_env.DynamicLaserEnv()
    summary = env.summary()
    image = env.render('rgb_array')
    env2 = laser_env.StaticLaserEnv.from_summary(summary)
    image2 = env2.render('rgb_array')
    assert env2.n_actions == env.n_actions
    assert env2.n_agents == env.n_agents
    assert env2.observation_shape == env.observation_shape
    assert env2.state_shape == env.state_shape
    assert env2.extra_feature_shape == env.extra_feature_shape
    assert np.array_equal(image, image2)


def test_restore_registry_dynamic():
    env = laser_env.DynamicLaserEnv()
    summary = env.summary()
    restored_env = rlenv.from_summary(summary)
    assert env.n_actions == restored_env.n_actions
    assert env.n_agents == restored_env.n_agents
    assert env.observation_shape == restored_env.observation_shape
    assert env.state_shape == restored_env.state_shape
    assert env.extra_feature_shape == restored_env.extra_feature_shape

def test_restore_registry_static():
    env = laser_env.StaticLaserEnv("lvl5")
    summary = env.summary()
    restored_env = rlenv.from_summary(summary)
    assert env.n_actions == restored_env.n_actions
    assert env.n_agents == restored_env.n_agents
    assert env.observation_shape == restored_env.observation_shape
    assert env.state_shape == restored_env.state_shape
    assert env.extra_feature_shape == restored_env.extra_feature_shape

def test_restore_dynamic_force_static():
    env = laser_env.DynamicLaserEnv()
    summary = env.summary()
    restored_env = rlenv.from_summary({**summary, "force_static": True})
    assert isinstance(restored_env, laser_env.StaticLaserEnv)
    assert env.n_actions == restored_env.n_actions
    assert env.n_agents == restored_env.n_agents
    assert env.observation_shape == restored_env.observation_shape
    assert env.state_shape == restored_env.state_shape
    assert env.extra_feature_shape == restored_env.extra_feature_shape
    