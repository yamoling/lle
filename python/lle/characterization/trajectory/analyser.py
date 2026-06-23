"""Build the temporal helper graph by replaying a trajectory.

The analyser replays a sequence of joint actions on a copy of the world and, at
every state the world passes through (the initial state and the state after each
action), records which agents help which others.

# Dependency detection (per state)
For every enabled laser beam of colour`c`:

* the beam is *blocked* iff agent`c` stands on one of its tiles (only an
  agent matching the laser colour can stand on the beam without dying);
* every *other* agent standing on a tile of that beam is alive precisely
  because the beam is blocked upstream, so it is being helped by agent`c`.

Each such situation yields a directed edge`c -> beneficiary` at the current
time step.  This is exactly the definition of *help* in LLE: agent`c` blocks a
laser of colour`c` and the beneficiary stands on the beam without dying.
"""

from __future__ import annotations

from collections import defaultdict

from lle.types import AgentId, LaserId
from lle.world import World

from .graph import TDG as TemporalDependencyGraph
from .types import Plan


def detect_dependencies(world: World) -> set[tuple[AgentId, AgentId]]:
    """
    Return the`(helper, beneficiary)` edges active in the world's current state.

    TODO: optimize this function to avoid re-constructing the beams dictionary at each call.
    """
    beams: dict[LaserId, list] = defaultdict(list)
    for laser in world.lasers:
        if not laser.is_disabled:
            beams[laser.laser_id].append(laser)

    edges: set[tuple[AgentId, AgentId]] = set()
    for tiles in beams.values():
        colour = tiles[0].agent_id
        beneficiaries: list[AgentId] = []
        blocker_present = False
        for tile in tiles:
            occupant = tile.agent
            if occupant is None:
                continue
            if occupant == colour:
                blocker_present = True
            else:
                # An alive agent of another colour can only stand here because
                # the beam is blocked upstream by the colour agent.
                beneficiaries.append(occupant)
        if blocker_present:
            for beneficiary in beneficiaries:
                edges.add((colour, beneficiary))
    return edges


def profile_trajectory(world: World, plan: Plan, *, reset: bool = True):
    """
    Compute the trajectory profile of the provided `plan` on the provided `world`.
    Check `TemporalDependencyGraph.from_plan` for more details about arguments.
    """
    graph = TemporalDependencyGraph.from_plan(plan, world, reset=reset)
    return graph.profile()
