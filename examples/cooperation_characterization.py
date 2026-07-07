"""Proof of concept: universal cooperation characterisation via SAT/UNSAT.

This script demonstrates :func:`lle.characterize`, which answers the *universal*
question "does **every** valid plan of length ≤ t force one agent to help
another?" — as opposed to :func:`lle.analyse_cooperation`, which inspects a
single concrete trajectory.

The proof-of-concept level:

    .   .  S0  S1  .   .
    L0E .   .   .  @   .
    .   .   .   .  .   .
    .   .   .   .  .   .
    X   X   .   .  .   .

Agent 0's red laser fires east along row 1. Agent 1 (a different colour) dies if
it crosses the beam, so for short plans agent 0 must step onto the beam to block
it and let agent 1 cross. Once enough time is available, agent 1 can instead
detour around the wall on the far right, and the dependency disappears.

Run with:  ``python examples/cooperation_characterization.py``
(requires ``maturin dev`` and the optional ``generator`` extra).
"""

from __future__ import annotations

from lle import World, characterize
from lle.solver.solver import solve, solve_no_cooperation

LEVEL = """
 .   .  S0  S1  .   .
L0E  .   .   .  @   .
 .   .   .   .  .   .
 .   .   .   .  .   .
 X   X   .   .  .   .
"""


def main() -> None:
    world = World(LEVEL)
    t_max = 12
    props = characterize(world, t_max=t_max)

    print("World characterization (t_max = {})".format(t_max))
    print("-" * 52)
    print(f"solution lower bound .......... {props.solution_lower_bound}")
    print(f"shortest valid plan ........... {props.first_solvable_length}")
    print(f"fully-independent threshold ... {props.fully_independent_threshold}")
    print()

    print("Independence thresholds (beneficiary <- helper):")
    for (beneficiary, helper), threshold in sorted(props.independence_threshold.items()):
        print(f"  agent {beneficiary} depends on agent {helper}: threshold = {threshold}")
    print()

    # The headline property: agent 1 needs agent 0's help only for short plans.
    print("Does agent 1 require agent 0's help for plans of length <= t?")
    for t in range(props.first_solvable_length or 0, t_max + 1):
        print(f"  t = {t:2d}: depends(1, 0, t) = {props.depends(1, 0, t)}")
    print()

    print("Does a fully cooperation-free plan of length <= t exist?")
    for t in range(props.first_solvable_length or 0, t_max + 1):
        print(f"  t = {t:2d}: is_independent(t) = {props.is_independent(t)}")
    print()

    # Relationship to the concrete solvers. Under strict (no-cooperation) laser
    # mode a different-colour agent can never stand on a beam, and in any valid
    # plan an agent only stands on a beam when the source agent blocks it upstream
    # (a help event). So "no blocking" and "no help at all" coincide, and the
    # global independence threshold equals the shortest no-cooperation plan length.
    plan = solve(world, t_max)
    no_coop = solve_no_cooperation(world, t_max=t_max)
    print("Cross-checks against the concrete solvers:")
    print(f"  shortest plan (any cooperation) ...... {len(plan) if plan else None}")
    print(f"  shortest no-cooperation plan ......... {len(no_coop) if no_coop else None}")
    print(f"  fully-independent threshold .......... {props.fully_independent_threshold}")
    assert props.fully_independent_threshold is not None and no_coop is not None
    assert props.fully_independent_threshold == len(no_coop), "no-cooperation solvability and full help-freedom must coincide"


if __name__ == "__main__":
    main()
