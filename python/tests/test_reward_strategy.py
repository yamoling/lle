from lle.env.reward_strategy import SingleObjective, MultiObjective, REWARD_DEATH, REWARD_GEM, REWARD_DONE, REWARD_EXIT
from lle import WorldEvent, EventType


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
