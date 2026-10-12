"""Build the temporal helper graph by replaying a trajectory.

The analyser replays a sequence of joint actions on a copy of the world and, at
every state the world passes through (the initial state and the state after each
action), records which agents help which others.

# Dependency detection (per state)
For every enabled laser beam of colour`c`:

* the beam is *blocked* iff agent`c` or a box stands on one of its tiles (only
  an agent matching the laser colour can stand on the beam without dying);
* every *other* agent standing on a tile of that beam with no box upstream is
  alive precisely because agent`c` blocks the beam upstream, so it is being
  helped by agent`c`. A beam blocked by a box is never help.

Each such situation yields a directed edge`c -> beneficiary` at the current
time step.  This is exactly the definition of *help* in LLE: agent`c` blocks a
laser of colour`c` and the beneficiary stands on the beam without dying.

# Lift help (per step)
An agent that takes `TRIGGER` on a button it may press helps every *other*
agent that a lift of the button's group moves during that step. The edge is
dated with the state reached after the step.
"""

from __future__ import annotations

from collections import defaultdict
from collections.abc import Sequence

from lle.types import AgentId, LaserId
from lle.world import Action, EventType, World, WorldEvent

from .graph import TemporalCooperationGraph
from .types import Plan


def detect_dependencies(world: World) -> set[tuple[AgentId, AgentId]]:
    """
    Return the`(helper, beneficiary)` laser edges active in the world's current state.

    TODO: optimize this function to avoid re-constructing the beams dictionary at each call.
    """
    beams: dict[LaserId, list] = defaultdict(list)
    for laser in world.lasers:
        if not laser.is_disabled:
            beams[laser.laser_id].append(laser)
    state = world.get_state()
    boxes = {pos for pos, present in zip(world.boxes_positions, state.boxes_present) if present}

    edges: set[tuple[AgentId, AgentId]] = set()
    for tiles in beams.values():
        colour = tiles[0].agent_id
        di, dj = tiles[0].direction.delta
        # Walk the beam downstream, from the source.
        tiles.sort(key=lambda tile: di * tile.pos[0] + dj * tile.pos[1])
        for tile in tiles:
            if tile.pos in boxes:
                # Everything downstream is protected by the box, not by the colour agent.
                break
            occupant = tile.agent
            if occupant is not None and occupant != colour:
                # An alive agent of another colour can only stand here because
                # the beam is blocked upstream by the colour agent.
                edges.add((colour, occupant))
    return edges


def detect_lift_help(world: World, joint_action: Sequence[Action], events: Sequence[WorldEvent]) -> set[tuple[AgentId, AgentId]]:
    """
    Return the `(helper, beneficiary)` lift edges of the step that just produced `events`: the helper
    pressed a button whose group lifted the beneficiary. `world` is the state after the step.
    """
    lift_groups = {lift.pos: lift.group_id for lift in world.lifts}
    colours = world.agent_colours
    positions = world.agents_positions
    edges: set[tuple[AgentId, AgentId]] = set()
    for event in events:
        if event.event_type != EventType.LIFT_MOVED or event.agent_id is None or event.from_position is None:
            continue
        group = lift_groups[event.from_position]
        for button in world.buttons:
            if button.group_id != group:
                continue
            # A presser stays on its button during the step.
            for helper, pos in enumerate(positions):
                if (
                    pos == button.pos
                    and helper != event.agent_id
                    and joint_action[helper] == Action.TRIGGER
                    and button.authorized_colour in (None, colours[helper])
                ):
                    edges.add((helper, event.agent_id))
    return edges


def profile_plan(world: World, plan: Plan, *, reset: bool = True):
    """
    Compute the profile of `plan` on the provided `world`.
    Check `TemporalDependencyGraph.from_plan` for more details about arguments.
    """
    graph = TemporalCooperationGraph.from_plan(plan, world, reset=reset)
    return graph.profile()
