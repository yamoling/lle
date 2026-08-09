from __future__ import annotations

import lle
from lle import World
from lle.solver import SolveMode

# A map where the two facing length-2 beams must each be crossed (interdependence) on the short
# route through columns 0-1, but a longer detour exists down the laser-free highway in columns
# 4-5 (reached over the top row, around the wall column `@`). So interdependent(2) cooperation is *required*
# only while the time budget is too small to take the detour.
TIME_DEPENDENT = """
 . S0 S1 . . .
L0E .  . @ . .
L1E .  . @ . .
 .  .  . @ . .
 .  X  X . . .
"""

# Two facing beams spanning the whole corridor: there is no detour, so cooperation is
# required at every solvable horizon.
ALWAYS_INTERDEPENDENT = """
 S0 . . S1
L0E . . .
 .  . . L1W
 X  . . X
"""


def report(name: str, world: World, t_max: int) -> None:
    print(f"\n=== {name} (t_max={t_max}) ===")
    for t in range(t_max + 1):
        if lle.solve(world, t) is None:
            continue  # no plan of this length at all
        free = lle.solve(world, t, mode=SolveMode.no_interdependence(2)) is not None
        verdict = "is free of interdependent help" if free else "requires interdendence"
        print(f"  t={t:2}: solvable, exact-length plan {verdict}")
    print(f"  -> requires_interdependence(t_max={t_max}) = {lle.characterize(world, t_max).is_interdependent(2)}")


def main() -> None:
    # Canonical levels: 1 is independent, 3 is *asymmetric* cooperation),
    # 6 genuinely needs two agents to help each other.
    report("level 1 (independent)", World.level(1), 10)
    report("level 3 (asymmetric coop)", World.level(3), 12)
    report("level 6 (interdependent-2)", World.level(6), 21)

    # `None` proves every plan within this horizon contains a temporal help chain of length 2.
    chain_free = lle.solve(World.level(6), 21, mode=SolveMode.no_chain(2))
    print(f"\\nlevel 6 has a chain-free plan: {chain_free is not None}")

    report("always-interdependent corridor", World(ALWAYS_INTERDEPENDENT), 10)
    report("time-dependent", World(TIME_DEPENDENT), 16)


if __name__ == "__main__":
    main()
