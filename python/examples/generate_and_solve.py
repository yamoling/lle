import lle
from lle import CooperationLevel

if __name__ == "__main__":
    # 1. Generate a solvable, cooperation-requiring world (SAT-verified).
    world = lle.generate("level6_style")
    env = lle.LLE(world)

    # 2. Check it really requires cooperation.
    assert lle.is_cooperative(world)  # True

    # 3. Solve it: returns a joint plan, or None if unsolvable.
    plan = lle.solve(world, t_max=21)
    if plan is None:
        print("No solution found !")
        exit()
    world.reset()
    for joint_action in plan:  # list[tuple[Action, ...]]
        world.step(list(joint_action))  # replays straight on the World

    # Other kinds:
    lle.generate(kind="random", width=5, height=15, n_agents=2, cooperative=True)
    lle.generate(kind="constructive", width=7, height=7, n_agents=2, n_lasers=2)

    # 4. Inspect the *precise* cooperation shape with `cooperation_level`.
    #    Returns one of the CooperationLevel members; see its docstring for
    #    the structural meaning of each (UNSOLVABLE, INDEPENDENT, COOPERATIVE,
    #    ASYMMETRIC, MUTUAL, CHAIN, DISTRIBUTED, FULLY_COUPLED).
    level = lle.cooperation_level(world, t_max=21)
    print(f"level6_style world is classified as: {level.value}")
    assert level in CooperationLevel.cooperative_subtypes()

    # 5. Ask the generator for a *specific* cooperation shape via `profile`.
    #    Only valid when `cooperative=True`. The generator keeps sampling until
    #    it finds a world whose classification matches the requested profile.
    asymmetric_world = lle.generate(
        kind="constructive",
        width=6,
        height=6,
        n_agents=2,
        n_lasers=1,
        cooperative=True,
        profile=CooperationLevel.ASYMMETRIC,
        t_max=15,
        seed=0,
    )
    assert lle.cooperation_level(asymmetric_world, t_max=15) is CooperationLevel.ASYMMETRIC
