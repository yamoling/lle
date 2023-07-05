import rlenv
import lle


def test_save_summary():
    env = lle.LLE.from_file("lvl6")
    summary = env.summary()
    env2 = lle.LLE.from_summary(summary)
    assert env2.n_actions == env.n_actions
    assert env2.n_agents == env.n_agents
    assert env2.observation_shape == env.observation_shape
    assert env2.state_shape == env.state_shape
    assert env2.extra_feature_shape == env.extra_feature_shape


def test_restore_registry():
    env = lle.LLE.from_file("lvl3")
    summary = env.summary()
    restored_env = rlenv.from_summary(summary)
    assert env.n_actions == restored_env.n_actions
    assert env.n_agents == restored_env.n_agents
    assert env.observation_shape == restored_env.observation_shape
    assert env.state_shape == restored_env.state_shape
    assert env.extra_feature_shape == restored_env.extra_feature_shape
