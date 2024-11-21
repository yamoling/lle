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

    done = env.step([Action.EAST.value, Action.STAY.value]).done
    assert done
