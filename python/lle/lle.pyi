from typing import Tuple, List
import numpy as np

class World:
    def __init__(self, filename: str):
        """Constructor. Accepts either level names or filenames.
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
    def step(self, actions: List[Action]) -> float:
        """Perform an action for each agent in the world."""
    def reset(self):
        """Reset the world to its initial state."""
    def available_actions(self) -> List[List[Action]]:
        """Return the list of available actions at the current time step for each agent."""
    def get_image(self) -> List[np.uint8]:
        """Return a rendered image of the world"""

class Action:
    NORTH = 0
    SOUTH = 1
    EAST = 2
    WEST = 3
    STAY = 4

    N: int
    """Enum cardinality"""

    @property
    def delta(self) -> Tuple[int, int]:
        """The change (i, j) in coordinates for this action."""
    @property
    def value(self) -> int:
        """The integer value of this action."""
