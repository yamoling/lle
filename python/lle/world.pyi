from typing import Tuple, List, Any, final
import numpy as np

from .action import Action
from .agent import Agent
from .tiles import Gem, LaserSource, Laser
from .types import Position

@final
class WorldState:
    def __init__(self, agent_positions: List[Position], gems_collected: List[bool]):
        """Construct a WorldState from the position of each agent and the collection status of each gem."""
    @property
    def agents_positions(self) -> List[Position]:
        """The position of each agent."""
    @property
    def gems_collected(self) -> List[bool]:
        """The collection status of each gem."""
    def __hash__(self) -> int: ...
    def __eq__(self, __value: object) -> bool: ...

@final
class World:
    def __init__(self, world_str: str):
        """Constructs a World from a string.
        Raises:
            - RuntimeError if the file is not a valid level.
            - ValueError if the file is not a valid level (inconsistent dimensions or invalid grid).
        """
    @property
    def n_agents(self) -> int:
        """The number of agents in the world."""
    @property
    def done(self) -> bool:
        """
        Whether the game is over, i.e. agents can no longer perform actions.
        This happens when an agent has died or when all agents are on exit tiles.
        """
    @property
    def width(self) -> int:
        """The width (in number of tiles) of the gridworld."""
    @property
    def height(self) -> int:
        """The height (in number of tiles) of the gridworld."""
    @property
    def image_dimensions(self) -> Tuple[int, int]:
        """The dimensions (in pixels) of the image redered (width, height)"""
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
    def void_pos(self) -> List[Position]:
        """The position of every void tile."""
    @property
    def laser_sources(self) -> List[Tuple[Position, LaserSource]]:
        """The position of every laser source."""
    @property
    def lasers(self) -> List[Tuple[Position, Laser]]:
        """The position of every laser."""
    @property
    def agents_positions(self) -> List[Position]:
        """Return the list of agent positions"""
    @property
    def world_string(self) -> str:
        """The string upon which the world was constructed."""
    @property
    def exit_rate(self) -> float:
        """The ratio of agents that have exited the world (i.e. enter the elevator)"""
    def step(self, actions: List[Action]) -> float:
        """
        Perform an action for each agent in the world and return the collective step reward.

        Unless at least an agent dies, the step reward is the sum of the following:
            - REWARD_GEM_COLLECTED for each gem collected at this time step
            - REWARD_AGENT_JUST_ARRIVED for each agent that has reached an exit tile
            - REWARD_END_GAME if all agents have exited the world
        If an agent dies, the step reward is REWARD_AGENT_DIED times the number of dead agents.
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
        """Retrieve the standard level (bewteen `1` and `6`)."""
