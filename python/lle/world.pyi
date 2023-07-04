from typing import Tuple, List
from lle import Position
import numpy as np

from .action import Action
from .agent import Agent
from .tiles import Gem, LaserSource, Laser

class World:
    def __init__(self, filename: str):
        """Constructor. Accepts either level names (level[1-6]) or filenames for custom maps.
        Raises:
            - FileNotFoundError if the file does not exist.
            - RuntimeError if the file is not a valid level.
            - ValueError if the file is not a valid level (inconsistent dimensions or invalid grid).
        """
    @property
    def n_agents(self) -> int:
        """The number of agents in the world."""
    @property
    def done(self) -> bool:
        """Whether the game is over, i.e. agents can no longer perform actions.
        This happens when an agent is dead or all agents are on fini tiles."""
    @property
    def width(self) -> int:
        """The width of the gridworld."""
    @property
    def height(self) -> int:
        """The height of the gridworld."""
    @property
    def image_dimensions(self) -> Tuple[int, int]:
        """The dimensions of the image redered (width, height)"""
    @property
    def gems_collected(self) -> int:
        """The number of gems collected by the agents so far in the episode."""
    @property
    def n_gems(self) -> int:
        """The total number of gems in the world."""
    @property
    def agents(self) -> List[Agent]:
        """
        The list of agents in the world.

        Note: This operation is rather costly because the agents are copied everytime this
        property is accessed.
        """
    @property
    def gems(self) -> List[Tuple[Position, Gem]]:
        """The list of gems in the world."""
    @property
    def exit_pos(self) -> List[Position]:
        """The list of exit positions for each agent."""
    @property
    def wall_pos(self) -> List[Position]:
        """The position of every wall."""
    @property
    def laser_sources(self) -> List[Tuple[Position, LaserSource]]:
        """The position of every laser source."""
    @property
    def lasers(self) -> List[Tuple[Position, Laser]]:
        """The position of every laser."""
    @property
    def agent_positions(self) -> List[Position]:
        """Return the list of agent positions"""
    def step(self, actions: List[Action]) -> float:
        """Perform an action for each agent in the world."""
    def reset(self):
        """Reset the world to its initial state."""
    def available_actions(self) -> List[List[Action]]:
        """Return the list of available actions at the current time step for each agent."""
    def get_image(self) -> np.ndarray[np.uint8]:
        """Return a rendered image of the world"""
    def exit_rate(self) -> float:
        """Return the rate at which agents exit the world (i.e. enter the elevator)"""
    @staticmethod
    def from_str(world_str: str) -> "World":
        """Create a world from a string representation of a level."""
