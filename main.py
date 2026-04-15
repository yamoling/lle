import lle
import logging

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

env = lle.from_file("lift.toml").build()
done = False
obs, state = env.reset()
while not done:
    env.render() # Uncomment to render
    actions = env.sample_action()
    step = env.step(actions)
    # Access the step data with `step.obs`, `step.reward`, ...
    done = step.is_terminal # Either done or truncated