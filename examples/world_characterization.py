"""Proof of concept: preventing *mutual* cooperation in the SAT solver.

Mutual cooperation between two agents `a` and `b` is the conjunction

    "a helps b cross one of a's laser beams at some point"   AND
    "b helps a cross one of b's laser beams at some point".

`lle.solve(world, t_max, "no-mutual-cooperation")` searches for the shortest plan in which *no*
pair of agents mutually cooperates.

Run with:  python python/examples/mutual_cooperation.py
"""

from __future__ import annotations

import lle
from lle import World

# A map where the two facing length-2 beams must each be crossed (mutual help) on the short
# route through columns 0-1, but a longer detour exists down the laser-free highway in columns
# 4-5 (reached over the top row, around the wall column `@`). So mutual cooperation is *required*
# only while the time budget is too small to take the detour.
TIME_DEPENDENT = """
 . S0 S1 . . .
L0E .  . @ . .
L1E .  . @ . .
 .  .  . @ . .
 .  X  X . . .
"""

# Two facing beams spanning the whole corridor: there is no detour, so mutual cooperation is
# required at every solvable horizon.
ALWAYS_MUTUAL = """
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
        free = lle.solve(world, t, mode="no-mutual-cooperation") is not None
        verdict = "free of mutual help" if free else "MUTUAL HELP REQUIRED"
        print(f"  t={t:2}: solvable, shortest plan is {verdict}")
    print(f"  -> requires_mutual_cooperation(t_max={t_max}) = {lle.characterize(world, t_max).is_mutual}")


def main() -> None:
    # Canonical levels: 1 is independent, 3 is *asymmetric* cooperation (one-directional, so NOT
    # mutual), 6 genuinely needs two agents to help each other.
    report("level 1 (independent)", World.level(1), 10)
    report("level 3 (asymmetric coop)", World.level(3), 12)
    report("level 6 (mutual)", World.level(6), 21)

    report("always-mutual corridor", World(ALWAYS_MUTUAL), 10)
    report("time-dependent mutual", World(TIME_DEPENDENT), 16)


if __name__ == "__main__":
    main()
