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
        """Construct a WorldState from the (i, j) position of each agent and the collection status of each gem."""

    agents_positions: List[Position]
    """The (i, j) position of each agent."""
    gems_collected: List[bool]
    """The collection status of each gem."""
    def __hash__(self) -> int: ...
    def __eq__(self, __value: object) -> bool: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __deepcopy__(self, memo: Any) -> "WorldState": ...

@final
class World:
    def __init__(self, world_str: str):
        """Constructs a World from a string.
        Raises:
            - RuntimeError if the file is not a valid level.
            - ValueError if the file is not a valid level (inconsistent dimensions or invalid grid).
        """
    def __deepcopy__(self, memo: Any) -> "World": ...
    agents_positions: List[Position]
    """The current (i, j) position of each agent"""
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
    gems: dict[Position, Gem]
    """
    The list of gems in the world
    
    Note: Accessing this attribute is costly because it creates the mapping on the fly every time.
    """
    start_pos: List[Position]
    """The (i, j) position of each start tile."""
    exit_pos: List[Position]
    """The (i, j) position of each exit tile."""
    wall_pos: List[Position]
    """The (i, j) position of every wall tile."""
    void_pos: List[Position]
    """The (i, j) position of every void tile."""
    laser_sources: dict[Position, LaserSource]
    """
    A mapping from positions to laser sources.
    
    Note: Accessing this attribute is costly because it creates the mapping on the fly every time.
    """
    lasers: List[Tuple[Position, Laser]]
    """
    The (i, j) position of every laser.

    Notes: 
        - Accessing this attribute is costly because it creates the list on the fly for every call.
        - Since two lasers can cross, there can be duplicates in the positions.
    """
    world_string: str
    """The string upon which the world has been constructed."""

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

        Raise:
            - `InvalidActionError` if an agent takes an action that is not available.
            - `ValueError` if the number of actions is different from the number of agents
        """
    def reset(self) -> None:
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
    def set_state(self, state: WorldState) -> list[WorldEvent]:
        """
        Force the world to a given state.

        - Returns the list of events that occurred while agents entered their state.
        - Raises a `InvalidWorldStateError` if the state is invalid.
        """

    @staticmethod
    def from_file(filename: str) -> "World":
        """
        Parse the content of `filename` to create a World.

        Raise a `FileNotFoundError` if the file does not exist.
        """
    @staticmethod
    def level(level: int) -> "World":
        """
        Retrieve the standard level (between `1` and `6`).
        A `ValueError` is raised if the level is invalid.
        """
