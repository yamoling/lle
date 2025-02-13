from lle import LLE

env = LLE.level(5).build()
done = False
obs, state = env.reset()
while not done:
    # env.render() # Uncomment to render
    actions = env.action_space.sample(env.available_actions())
    step = env.step(actions)
    done = step.is_terminal
