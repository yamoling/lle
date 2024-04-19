from lle import LLE

env = LLE.level(5).build()
done = truncated = False
obs = env.reset()
while not (done or truncated):
    # env.render() # Uncomment to render
    actions = env.action_space.sample(env.available_actions())
    obs, reward, done, truncated, info = env.step(actions)
