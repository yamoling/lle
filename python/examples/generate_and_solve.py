import lle

if __name__ == "__main__":
    # 1. Generate a solvable, cooperation-requiring world (SAT-verified).
    world = lle.generate("level6_style")
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

    # Other kinds:
    lle.generate(kind="random", width=5, height=15, n_agents=2, cooperation=True)
    lle.generate(kind="constructive", width=7, height=7, n_agents=2, n_lasers=2)
