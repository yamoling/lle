from lle import LLE, ObservationType, Action
from timeit import timeit

env = LLE.from_file("lvl6", ObservationType.FLATTENED)


def bench_reset():
    env.reset()


def bench_step(n: int):
    action_space = env.action_space
    i = 0
    done = True
    while i < n:
        if done:
            env.reset()
        _, _, done, _, _ = env.step([Action.STAY.value, Action.STAY.value, Action.STAY.value, Action.STAY.value])
        i += 1


def bench_image():
    env.render("rgb_array")


duration = timeit(bench_reset, number=10000)
print(f"Reset: {duration:.2f}ms")

duration = timeit(lambda: bench_step(100), number=100)
print(f"Step: {duration:.2f}ms")

duration = timeit(bench_image, number=1000)
print(f"Image: {duration:.2f}ms")


"""
Reset: 4.56ms
Step: 7.96ms
Image: 3.87ms
"""
