import lle

if __name__ == "__main__":
    # 1. Generate a solvable, cooperation-requiring world (SAT-verified).
    world = lle.generate(n_agents=2).cooperative(t_max=21).build()
    assert isinstance(world, lle.World)

    # 2. Check it really requires cooperation.
    assert lle.is_cooperative(world)

    # 3. Solve it: returns a joint plan, or None if unsolvable.
    plan = lle.solve(world, 21)
    if plan is None:
        print("No solution found !")
        exit()
    world.reset()
    for joint_action in plan:  # list[tuple[Action, ...]]
        world.step(joint_action)  # replays straight on the World

    # Other layouts:
    lle.generate(width=5, height=15, n_agents=2).lanes().cooperative().build()
    lle.generate(width=7, height=7, n_agents=2).lasers(2, span=5).build()
