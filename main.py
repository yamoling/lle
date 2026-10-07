import logging
from time import sleep

import lle
from lle import Action


def setup_logging():
    root = logging.getLogger()
    root.setLevel(logging.DEBUG)  # capture everything

    fileName = "lle_example.log"
    fileHandler = logging.FileHandler(fileName)
    fileHandler.setLevel(logging.DEBUG)  # capture everything to the file
    # Clear existing handlers (important in notebooks / frameworks)
    root.handlers.clear()
    root.addHandler(fileHandler)


setup_logging()

N, S, E, W, STAY, TRIGGER = (
    Action.NORTH,
    Action.SOUTH,
    Action.EAST,
    Action.WEST,
    Action.STAY,
    Action.TRIGGER,
)
solution = [
    (E, E),
    (E, E),
    (E, STAY),
    (STAY, N),
    (STAY, TRIGGER),
    (W, E),
    (TRIGGER, STAY),
    (W, S),
    (W, STAY),
    (S, STAY),
]

env = lle.from_file("lift.toml").build()
done = False
obs, state = env.reset()
for actions in solution:
    env.render()  # uncomment to render
    sleep(1)  # uncomment to slow down the rendering
    step = env.step([action.value for action in actions])
    # Access the step data with `step.obs`, `step.reward`, ...
    done = step.is_terminal  # Either done or truncated
    if done:
        break

assert done, "The hand-crafted solution did not resolve the level"
print(f"Solved lift.toml in {len(solution)} steps, final reward={step.reward}")
