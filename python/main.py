from lle import World, Action, REWARD_END_GAME, REWARD_AGENT_JUST_ARRIVED, REWARD_GEM_COLLECTED


world = World(
    """
    S0 X . .
    .  . . .
    G  . . .
    """
)


def play():
    """Collect the gem and finish the game. Check that the reward is is correct when collecting it."""
    world.reset()
    world.step([Action.SOUTH])
    reward = world.step([Action.SOUTH])
    assert reward == REWARD_GEM_COLLECTED
    assert not world.done
    r = world.step([Action.NORTH])
    assert r == 0
    r = world.step([Action.NORTH])
    assert r == 0
    reward = world.step([Action.EAST])
    assert world.done
    assert reward == REWARD_END_GAME + REWARD_AGENT_JUST_ARRIVED


for _ in range(10):
    play()
