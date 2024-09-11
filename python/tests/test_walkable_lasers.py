from lle import LLE, Action


def test_walkable_laser_enabled():
    # when walkable_lasers is True (default), agents can run in any laser
    env = LLE.from_str(
        """
@ @ L0S @  @
@ .  .  .  @
@ X  .  S0 @
@ X  .  S1 @
@ @  @  @  @
            """
    ).single_objective()
    env.reset()
    available_actions = env.available_actions()

    # Agent 0
    assert available_actions[0, Action.WEST.value]
    # Agent 1
    assert available_actions[1, Action.WEST.value]


def test_walkable_laser_disabled_laser_enabled():
    # when walkable_lasers is False, agents can't run in active lasers of different color
    env = (
        LLE.from_str(
            """
@ @ L0S @  @
@ .  .  .  @
@ X  .  S0 @
@ X  .  S1 @
@ @  @  @  @
            """
        )
        .walkable_lasers(False)
        .single_objective()
    )
    env.reset()
    available_actions = env.available_actions()

    # Agent 0
    assert available_actions[0, Action.WEST.value]
    # Agent 1
    assert not available_actions[1, Action.WEST.value]


def test_walkable_laser_disabled_laser_disabled():
    # when walkable_lasers is False, agents can run in disabled lasers of different color
    env = (
        LLE.from_str(
            """
@ @ L0S @  @
@ .  .  .  @
@ X  S0 .  @
@ X  .  S1 @
@ @  @  @  @
            """
        )
        .walkable_lasers(False)
        .single_objective()
    )
    env.reset()
    available_actions = env.available_actions()
    # Agent 1
    assert available_actions[1, Action.WEST.value]


# check with different laser id
def test_walkable_laser_disabled_laser_enabled2():
    # switched laser id
    env = (
        LLE.from_str(
            """
@ @ L1S @  @
@ .  .  .  @
@ X  .  S0 @
@ X  .  S1 @
@ @  @  @  @
            """
        )
        .walkable_lasers(False)
        .single_objective()
    )
    env.reset()
    available_actions = env.available_actions()

    # Agent 0
    assert not available_actions[0, Action.WEST.value]
    # Agent 1
    assert available_actions[1, Action.WEST.value]


def test_walkable_laser_disabled_laser_disabled2():
    # switched laser id
    env = (
        LLE.from_str(
            """
@ @ L1S @  @
@ .  .  .  @
@ X  S1 .  @
@ X  .  S0 @
@ @  @  @  @
            """
        )
        .walkable_lasers(False)
        .single_objective()
    )
    env.reset()
    available_actions = env.available_actions()
    # Agent 0
    assert available_actions[0, Action.WEST.value]
