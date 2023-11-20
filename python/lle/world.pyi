from typing import Tuple, List, Any, final
import numpy as np

from .event import WorldEvent
from .action import Action
from .agent import Agent
from .tile import Gem, LaserSource, Laser
from .types import Position

@final
class WorldState:
    def __init__(self, agents_positions: List[Position], gems_collected: List[bool]):
        """Construct a WorldState from the position of each agent and the collection status of each gem."""
    @property
    def agents_positions(self) -> List[Position]:
        """The position of each agent."""
    @property
    def gems_collected(self) -> List[bool]:
        """The collection status of each gem."""
    def __hash__(self) -> int: ...
    def __eq__(self, __value: object) -> bool: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __deepcopy__(self, memo: Any) -> "WorldState": ...

@final
class World:
    n_agents: int
    """The number of agents in the world."""
    width: int
    """The width (in number of tiles) of the gridworld."""
    height: int
    """The height (in number of tiles) of the gridworld."""
    image_dimensions: Tuple[int, int]
    """The dimensions (in pixels) of the image redered (width, height)"""
    n_gems: int
    """The total number of gems in the world."""
    gems: List[Tuple[Position, Gem]]
    """The list of gems in the world"""
    exit_pos: List[Position]
    """The positions of each exit tile."""
    wall_pos: List[Position]
    """The position of every wall tile."""
    void_pos: List[Position]
    """The position of every void tile."""
    laser_sources: List[Tuple[Position, LaserSource]]
    """The position of every laser source."""
    lasers: List[Tuple[Position, Laser]]
    """The position of every laser."""
    agents_positions: List[Position]
    """The current position of each agent"""
    world_string: str
    """The string upon which the world has been constructed."""

    def __init__(self, world_str: str):
        """Constructs a World from a string.
        Raises:
            - RuntimeError if the file is not a valid level.
            - ValueError if the file is not a valid level (inconsistent dimensions or invalid grid).
        """
    def __deepcopy__(self, memo: Any) -> "World": ...
    @property
    def gems_collected(self) -> int:
        """The number of gems collected by the agents so far in the episode."""
    @property
    def agents(self) -> List[Agent]:
        """
        The list of agents in the world.

        Note: This operation is rather costly because the agents are copied everytime this
        property is accessed.
        """
    def step(self, actions: List[Action]) -> list[WorldEvent]:
        """
        Perform an action for each agent in the world and return the list of
        events that occurred by peforming this step.
        """
    def reset(self):
        """Reset the world to its initial state."""
    def available_actions(self) -> List[List[Action]]:
        """
        Return the list of available actions at the current time step for each agent.

        The actions available for agent `n` are given by `world.available_actions()[n]`.
        """
    def get_image(self) -> np.ndarray[np.uint8, Any]:
        """Return a rendered image of the world"""
    def get_state(self) -> WorldState:
        """Return a state representation of the world."""
    def set_state(self, state: WorldState):
        """
        Force the world to a given state.

        A ValueError is raised if the state is invalid.
        """
    @staticmethod
    def from_file(filename: str) -> "World":
        """Parse the content `filename` to create a World."""
    @staticmethod
    def level(level: int) -> "World":
        """Retrieve the standard level (between `1` and `6`)."""
