"""Build the temporal helper graph by replaying a trajectory.

The analyser replays a sequence of joint actions on a copy of the world and, at
every state the world passes through (the initial state and the state after each
action), records which agents help which others.

Dependency detection (per state)
--------------------------------
For every enabled laser beam of colour ``c``:

* the beam is *blocked* iff agent ``c`` stands on one of its tiles (only an
  agent matching the laser colour can stand on the beam without dying);
* every *other* agent standing on a tile of that beam is alive precisely
  because the beam is blocked upstream, so it is being helped by agent ``c``.

Each such situation yields a directed edge ``c -> beneficiary`` at the current
time step.  This is exactly the definition of *help* in LLE: agent ``c`` blocks a
laser of colour ``c`` and the beneficiary stands on the beam without dying.
"""

from __future__ import annotations

from collections import defaultdict
from collections.abc import Sequence
from copy import deepcopy

from ..types import AgentId, LaserId
from ..world import Action, World
from .graph import DependencyEdge, TemporalDependencyGraph

JointAction = Action | Sequence[Action]
Trajectory = Sequence[JointAction]


def detect_dependencies(world: World) -> set[tuple[AgentId, AgentId]]:
    """Return the ``(helper, beneficiary)`` edges active in the world's current state."""
    beams: dict[LaserId, list] = defaultdict(list)
    for laser in world.lasers:
        if laser.is_disabled:
            continue
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


def analyse_cooperation(
    world: World,
    trajectory: Trajectory,
    *,
    reset: bool = True,
) -> TemporalDependencyGraph:
    """Replay ``trajectory`` and build the temporal helper graph.

    Args:
        world: The world to analyse. It is **not** mutated; the analysis runs on
            a deep copy.
        trajectory: The sequence of joint actions to replay. Each element is
            either a single `Action` (for a single-agent world) or a sequence of
            one `Action` per agent.
        reset: Whether to reset the copied world before replaying. Keep the
            default unless the trajectory is meant to continue from the world's
            current state.

    Returns:
        The `TemporalDependencyGraph` whose edges are the help relationships
        observed at the initial state (``t = 0``) and after each action
        (``t = 1, 2, ...``).
    """
    world = deepcopy(world)
    if reset:
        world.reset()

    edges: list[DependencyEdge] = []

    def record(t: int) -> None:
        for helper, beneficiary in detect_dependencies(world):
            edges.append(DependencyEdge(helper, beneficiary, t))

    record(0)
    for step_index, joint_action in enumerate(trajectory, start=1):
        world.step(joint_action)
        record(step_index)

    return TemporalDependencyGraph(world.n_agents, edges, horizon=len(trajectory))
