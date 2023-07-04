from lle import LLE, Action
import numpy as np


# def test_dynamic_env_reset():
#     env1 = DynamicLaserEnv(num_lasers=2)
#     env = TimeLimitWrapper(env1, 10)

#     def run(i):
#         print(i)
#         done = False
#         obs = env.reset()
#         while not done:
#             actions = env.action_space.sample(obs.available_actions)
#             obs, _, done, _ = env.step(actions)

#     for i in range(50):
#         run(i)


def test_available_actions():
    env = LLE("python/tests/maps/5x5_laser_2agents")
    env.reset()
    available_actions = env.get_avail_actions()
    # Agent 0
    assert available_actions[0, Action.NORTH.value] == 1
    assert available_actions[0, Action.EAST.value] == 0
    assert available_actions[0, Action.SOUTH.value] == 0
    assert available_actions[0, Action.WEST.value] == 1
    assert available_actions[0, Action.STAY.value] == 1

    # Agent 1
    assert available_actions[1, Action.NORTH.value] == 0
    assert available_actions[1, Action.EAST.value] == 0
    assert available_actions[1, Action.SOUTH.value] == 0
    assert available_actions[1, Action.WEST.value] == 1
    assert available_actions[1, Action.STAY.value] == 1


def test_available_actions2():
    env = LLE("python/tests/maps/7x7_available_actions.txt")
    obs = env.reset()

    def check_available_actions(available: np.ndarray[np.int32], expected_available: list[list[Action]]) -> bool:
        available_actions = np.zeros((2, Action.N), dtype=np.int32)
        for agent_id, actions in enumerate(expected_available):
            for action in actions:
                available_actions[agent_id, action.value] = 1
        return np.array_equal(available, available_actions)

    assert check_available_actions(
        obs.available_actions, [[Action.SOUTH, Action.WEST, Action.STAY], [Action.SOUTH, Action.EAST, Action.STAY]]
    )

    # Move the agent to the end location and check the available actions
    for _ in range(3):
        obs, *_ = env.step([Action.SOUTH.value, Action.SOUTH.value])
        check_available_actions(
            obs.available_actions,
            [[Action.NORTH, Action.SOUTH, Action.WEST, Action.STAY], [Action.NORTH, Action.SOUTH, Action.EAST, Action.STAY]],
        )
    obs, *_ = env.step([Action.SOUTH.value, Action.SOUTH.value])
    check_available_actions(obs.available_actions, [[Action.STAY] * 2])


def test_width_height():
    env = LLE("python/tests/maps/3x3.txt")
    assert env.width == 3
    assert env.height == 3

    env = LLE("python/tests/maps/3x4_gem.txt")
    assert env.width == 4
    assert env.height == 3


def test_state():
    env = LLE("python/tests/maps/3x3.txt")
    assert env.state_shape == (1 * 5 * 3 * 3,)
    env.reset()
    state = env.get_state()
    assert state.shape == env.state_shape


def test_env_summary_file_content_static():
    env = LLE("python/tests/maps/3x3_alternating")
    with open("python/tests/maps/3x3_alternating") as f:
        file_content = f.read()
    summary = env.summary()
    assert summary["map_file_content"] == file_content
    static_summary = env.summary(static=True)
    expected_static = "L0E . @\nS0 . G\nV . F"
    assert static_summary["map_file_content"] == expected_static


def test_action_meanings():
    env = LLE("python/tests/maps/3x3.txt")
    assert env.action_meanings == [a.name for a in Action]
