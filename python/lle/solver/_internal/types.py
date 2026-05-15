"""Value types used internally by the solver."""

from __future__ import annotations

from dataclasses import dataclass

from lle import World

Position = tuple[int, int]


@dataclass(frozen=True)
class AgentData:
    color: int
    position: Position


@dataclass(frozen=True)
class LaserSourceData:
    color: int
    direction: tuple[int, int]
    position: Position


def agents_from_world(world: World) -> list[AgentData]:
    return [AgentData(color=i, position=pos) for i, pos in enumerate(world.start_pos)]


def laser_sources_from_world(world: World) -> list[LaserSourceData]:
    return [
        LaserSourceData(
            color=src.agent_id,
            direction=src.direction.delta(),
            position=src.pos,
        )
        for src in world.laser_sources
    ]
