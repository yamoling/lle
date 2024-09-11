from lle import LLE, Action


def test_end_strategy():
    env = (
        LLE.from_str(
            """
S0  G  X
S1 L1N X
"""
        )
        .death_strategy("end")
        .single_objective()
    )
    env.reset()

    obs, r, done, trunc, info = env.step([Action.EAST.value, Action.STAY.value])
    assert done


def test_stay_strategy():
    env = (
        LLE.from_str(
            """
S0 .  G  X
S1 . L1N X
"""
        )
        .death_strategy("stay")
        .single_objective()
    )
    env.reset()
    env.step([Action.EAST.value, Action.STAY.value])
    obs, r, done, trunc, info = env.step([Action.EAST.value, Action.NORTH.value])
    assert not done
    assert env.world.gems_collected == 0
    # Check that the agent is back to its previous position
    assert env.world.agents_positions[0] == (0, 1)
    assert env.world.agents_positions[1] == (0, 0)
