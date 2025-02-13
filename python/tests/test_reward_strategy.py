from lle.env.reward_strategy import SingleObjective, MultiObjective, REWARD_DEATH, REWARD_GEM, REWARD_DONE, REWARD_EXIT, PotentialShapedLLE
from lle import WorldEvent, EventType, World, LLE, Action
import numpy as np


def test_single_objective():
    s = SingleObjective(2)
    s.reset()
    assert s.compute_reward([WorldEvent(EventType.GEM_COLLECTED, 0)]).item() == REWARD_GEM
    assert (
        s.compute_reward(
            [
                WorldEvent(EventType.AGENT_EXIT, 0),
                WorldEvent(EventType.AGENT_EXIT, 1),
            ]
        ).item()
        == REWARD_EXIT * 2 + REWARD_DONE
    )
    assert s.n_arrived == 2

    s.reset()
    assert s.compute_reward([WorldEvent(EventType.AGENT_DIED, 0)]).item() == REWARD_DEATH
    assert s.n_deads == 1

    s.reset()
    assert s.compute_reward([WorldEvent(EventType.AGENT_DIED, 0)]).item() == REWARD_DEATH
    assert s.compute_reward([WorldEvent(EventType.AGENT_DIED, 1)]).item() == REWARD_DEATH
    assert s.n_deads == 2
    assert s.n_arrived == 0


def test_multi_objective():
    IDX_GEM = MultiObjective.RW_GEM_IDX
    IDX_EXIT = MultiObjective.RW_EXIT_IDX
    IDX_DONE = MultiObjective.RW_DONE_IDX
    IDX_DEATH = MultiObjective.RW_DEATH_IDX

    s = MultiObjective(2)
    s.reset()
    assert s.compute_reward([WorldEvent(EventType.GEM_COLLECTED, 0)])[IDX_GEM] == REWARD_GEM
    r = s.compute_reward(
        [
            WorldEvent(EventType.AGENT_EXIT, 0),
            WorldEvent(EventType.AGENT_EXIT, 1),
        ]
    )
    assert r[IDX_EXIT] == REWARD_EXIT * 2
    assert r[IDX_DONE] == REWARD_DONE
    assert s.n_arrived == 2

    s.reset()
    assert s.compute_reward([WorldEvent(EventType.AGENT_DIED, 0)])[IDX_DEATH] == REWARD_DEATH
    assert s.n_deads == 1

    s.reset()
    assert s.compute_reward([WorldEvent(EventType.AGENT_DIED, 0)])[IDX_DEATH] == REWARD_DEATH
    assert s.compute_reward([WorldEvent(EventType.AGENT_DIED, 1)])[IDX_DEATH] == REWARD_DEATH
    assert s.n_deads == 2
    assert s.n_arrived == 0


def test_pbrs_single_objective():
    world = World("""
                S0 .  .
                .  . L0W
                X  .  .""")
    world.reset()
    s = SingleObjective(world.n_agents)
    pbrs = PotentialShapedLLE(s, world, 0.99, 0.5, world.laser_sources)

    # Step eastwards, there should be no reward issued
    events = world.step(Action.EAST)
    base_reward = s.compute_reward(events)[0]
    pbrs_reward = pbrs.compute_reward(events)[0]
    assert base_reward == 0.0
    assert pbrs_reward == pbrs.gamma * pbrs.reward_value - pbrs.reward_value

    # Step south in the laser, a shaped reward should be received
    events = world.step(Action.SOUTH)
    base_reward = s.compute_reward(events)[0]
    pbrs_reward = pbrs.compute_reward(events)[0]
    assert abs(base_reward - pbrs_reward) == pbrs.reward_value * pbrs.gamma

    events = world.step(Action.SOUTH)
    pbrs_reward = pbrs.compute_reward(events)[0]
    assert pbrs_reward == 0.0


def test_pbrs_raises_value_error():
    # This succeeds
    LLE.from_str("""
                S0 .  .
                .  . L0W
                X  .  .""").multi_objective().pbrs()

    try:
        # But this fails
        LLE.from_str("""
                    S0 .  .
                    .  . L0W
                    X  .  .""").pbrs().multi_objective()
        assert False, "Should have raised a ValueError because 'single_objective()' or 'multi_objective' was not called before 'pbrs()'"
    except ValueError:
        pass


def test_pbrs_multi_objective():
    world = World("""
                S0 .  .
                .  . L0W
                X  .  .""")
    world.reset()
    s = MultiObjective(world.n_agents)
    pbrs = PotentialShapedLLE(s, world, 0.99, 0.5, world.laser_sources)

    # Step eastwards, there should be no reward issued
    events = world.step(Action.EAST)
    base_reward = s.compute_reward(events)
    pbrs_reward = pbrs.compute_reward(events)
    assert np.array_equal(base_reward, [0.0] * 4)
    expected_shaped_reward = pbrs.gamma * pbrs.reward_value - pbrs.reward_value
    expected = np.array([0.0] * 4 + [expected_shaped_reward], dtype=np.float32)
    assert np.allclose(pbrs_reward, expected)

    # Step south in the laser, a shaped reward should be received
    events = world.step(Action.SOUTH)
    base_reward = s.compute_reward(events)
    pbrs_reward = pbrs.compute_reward(events)

    expected_pbrs = pbrs.reward_value * pbrs.gamma
    expected = np.array([0.0] * 4 + [expected_pbrs], dtype=np.float32)
    assert np.allclose(pbrs_reward, expected)

    events = world.step(Action.SOUTH)
    pbrs_reward = pbrs.compute_reward(events)[-1]
    assert pbrs_reward == 0.0


def test_pbrs_with_lle():
    env = (
        LLE.from_str("""
                S0 .  .
                .  . L0W
                X  .  .""")
        .pbrs(gamma=0.99, reward_value=0.5)
        .build()
    )
    env.reset()
    step = env.step([Action.EAST.value])
    assert step.reward[0] == 0.5 * 0.99 - 0.5
    step = env.step([Action.SOUTH.value])
    assert step.reward[0] == 0.5 * 0.99 - 0
    step = env.step([Action.SOUTH.value])
    assert step.reward[0] == 0.0
