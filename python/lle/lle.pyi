from . import exceptions
# This file is automatically generated by pyo3_stub_gen
# ruff: noqa: E501, F401

import lle.tiles
import numpy
import numpy.typing
import typing
from . import tiles
from enum import Enum, auto

__version__: str
class Agent:
    r"""
    An agent in the world.
    """
    num: int
    """The agent id."""
    is_dead: bool
    """Whether the agent is dead or not."""
    is_alive: bool
    """Whether the agent is alive or not."""
    has_arrived: bool
    """Whether the agent has reached an exit or not."""

class World:
    r"""
    The `World` represents the environment in which the agents evolve.
    A world is created from a string where each character represents a tile.
    There are 6 predefined levels for convenience.
    
    ```python
    from lle import World
    # Create from a predefined level
    w1 = World.level(5)
    # Create from a file
    w2 = World.from_file("my_map.txt")
    # Create from a string
    w3 = World("S0 X")
    ```
    """
    exit_pos: list[tuple[int, int]]
    """The positions of the exits tiles."""
    start_pos: list[tuple[int, int]]
    """The positions of the start tiles."""
    wall_pos: list[tuple[int, int]]
    """The positions of the walls."""
    void_pos: list[tuple[int, int]]
    """The positions of the void tiles."""
    height: int
    """The height of the world (in number of tiles)."""
    width: int
    """The width of the world (in number of tiles)."""
    n_gems: int
    """The number of gems in the world."""
    n_agents: int
    """The number of agents in the world."""
    world_string: str
    """The string upon which the world has been constructed."""
    image_dimensions: tuple[int, int]
    """The dimensions (in pixels) of the image redered (width, height)"""
    gems_collected: int
    """The number of gems collected by the agents so far since the last reset."""
    agents_positions: list[tuple[int, int]]
    """The (i, j) position of each agent."""
    gems:  dict[tuple[int, int], tiles.Gem]
    """The gems with their respective position."""
    lasers:  list[tuple[tuple[int, int], tiles.Laser]]
    """The (i, j) position of every laser.
    Since two lasers can cross, there can be duplicates in the positions."""
    laser_sources:  dict[tuple[int, int], tiles.LaserSource]
    """A mapping from (i, j) positions to laser sources."""
    agents: list[Agent]
    """The list of agents in the world."""
    def __new__(cls,map_str:str): ...
    @staticmethod
    def from_file(filename:str) -> World:
        r"""
        Parse the content of `filename` to create a World.
        Raises:
            FileNotFoundError: if the file does not exist.
        """
        ...

    @staticmethod
    def level(level:int) -> World:
        r"""
        Retrieve the standard level (between `1` and `6`).
        Raises:
            ValueError: if the level is invalid.
        """
        ...

    def step(self, action: Action | list[Action]) -> list[WorldEvent]:
        r"""
        Simultaneously perform an action for each agent in the world.
        Performing a step generates events (see `WorldEvent`) to give information about the consequences of the joint action.
        
        Args:
           action: The action to perform for each agent. A single action is also accepted if there is a single agent in the world.
        
        Returns:
          The list of events that occurred while agents took their action.
        
        Raises:
            `InvalidActionError` if an agent takes an action that is not available.
            `ValueError` if the number of actions is different from the number of agents
        
        Example:
        ```python
        world = World("S1 G X S0 X")
        world.reset()
        events = world.step([Action.STAY, Action.EAST])
        assert len(events) == 1
        assert events[0].agent_id == 1
        assert events[0].event_type == EventType.GEM_COLLECTED
        
        events = world.step([Action.EAST, Action.EAST])
        assert len(events) == 2
        assert all(e.event_type == EventType.AGENT_EXIT for e in events)
        ```
        """
        ...

    def reset(self) -> None:
        r"""
        Reset the world to its original state.
        This should be done directly after creating the world.
        """
        ...

    def available_actions(self) -> list[list[Action]]:
        r"""
        Compute the list of available actions at the current time step for each agent.
        The actions available for agent `n` are given by `world.available_actions()[n]`.
        Returns:
           The list of available actions for each agent.
        """
        ...

    def available_joint_actions(self) -> list[list[Action]]:
        r"""
        Compute the list of available joint actions at the current time step.
        The result has shape (x, n_agents) where x is the number of joint actions available.
        Returns:
          The list of available joint actions.
        
        Example:
        ```python
        world = World(". .  .  . .\n. S0 . S1 .\n. X  .  X .\n")
        world.reset()
        assert len(world.available_joint_actions()) == len(Action.ALL) ** 2
        ```
        """
        ...

    def get_image(self) -> numpy.typing.NDArray[numpy.uint8]:
        r"""
        Renders the world as an image and returns it in a numpy array.
        Returns:
            The image of the world as a numpy array of shape (height * 32, width * 32, 3) with type uint8.
        """
        ...

    def set_state(self, state:WorldState) -> list[WorldEvent]:
        r"""
        Force the world to a given state
        Args:
            state: The state to set the world to.
        Returns:
            The list of events that occurred while agents entered their state.
        Raises:
            InvalidWorldStateError: if the state is invalid.
        """
        ...

    def get_state(self) -> WorldState:
        r"""
        Return the current state of the world.
        """
        ...

    def __deepcopy__(self, _memo:dict) -> World:
        r"""
        Returns a deep copy of the object.
        
        Example:
        ```python
        from copy import deepcopy
        world = World("S0 X")
        world.reset()
        world_copy = deepcopy(world)
        world.step(Action.EAST)
        assert world.get_state() != world_copy.get_state()
        ```
        """
        ...

    def __getnewargs__(self) -> typing.Any:
        r"""
        This method is called to instantiate the object before deserialisation.
        It required "default arguments" to be provided to the __new__ method
        before replacing them by the actual values in __setstate__.
        """
        ...

    def __getstate__(self) -> tuple[str, list[bool], list[tuple[int, int]], list[bool]]:
        r"""
        Enable serialisation with pickle
        """
        ...

    def __setstate__(self, state:tuple[str, typing.Sequence[bool], typing.Sequence[tuple[int, int]], typing.Sequence[bool]]) -> None:
        r"""
        Enable deserialisation with pickle
        """
        ...

    def __repr__(self) -> str:
        ...


class WorldBuilder:
    width: int
    height: int
    n_agents: int
    start_positions: dict[int, tuple[int, int]]
    exit_positions: set[tuple[int, int]]
    available_positions: set[tuple[int, int]]
    def __new__(cls,width:int, height:int, n_agents:int): ...
    def build(self) -> World:
        ...

    def can_build(self) -> bool:
        ...

    def world_str(self) -> str:
        ...

    def set_start(self, pos:tuple[int, int], agent_num:int) -> None:
        ...

    def add_laser_source(self, pos:tuple[int, int], agent_id:int, direction: tiles.Direction) -> None:
        ...

    def add_wall(self, pos:tuple[int, int]) -> None:
        ...

    def add_exit(self, pos:tuple[int, int]) -> None:
        ...

    def add_gem(self, pos:tuple[int, int]) -> None:
        ...

    def reset(self) -> None:
        ...

    def clear(self, pos:tuple[int, int]) -> None:
        ...


class WorldEvent:
    event_type: EventType
    agent_id: int
    def __str__(self) -> str:
        ...

    def __repr__(self) -> str:
        ...


class WorldState:
    r"""
    A state in the `World` is defined by:
     - The position of each agent.
     - Whether each gem has been collected.
     - Whether each agent is alive.
    
    **Using `WorldState`s:**
    ```python
    from lle import WorldState, World
    w = World("S0 . X")
    w.reset()
    s1 = w.get_state()
    s2 = WorldState([(0, 1), [], [True]])
    world.set_state(s2)
    ```
    
    
    **Inheritance:**
    To inherit from `WorldState`, it is required to override the __new__ method such that it
    accepts **the same arguments** as the __init__ method in the same order (except `cls` instead of `self`).
    You should ignore the additional arguments in the __new__ method as shown below.
    ```python
    class SubWorldState(WorldState):
       def __init__(self, x: int, agents_positions: list[tuple[int, int]], gems_collected: list[bool], agents_alive: List[bool] | None = None):
           super().__init__(agents_positions, gems_collected, agents_alive)
           self.x = x
    
       def __new__(cls, _x: int, agents_positions: list[tuple[int, int]], gems_collected: list[bool], agents_alive: list[bool] | None = None):
           instance = super().__new__(cls, agents_positions, gems_collected, agents_alive)
           return instance
    ```
    """
    agents_positions: list[tuple[int, int]]
    """The position of each agent."""
    gems_collected: list[bool]
    """The collection status of each gem."""
    agents_alive: list[bool]
    """The status of each agent."""
    def __new__(cls,agents_positions,gems_collected,agents_alive = ...): ...
    def __init__(self, agents_positions:typing.Sequence[tuple[int, int]], gems_collected:typing.Sequence[bool], agents_alive:typing.Optional[typing.Sequence[bool]]) -> None:
        ...

    def as_array(self) -> typing.Any:
        ...

    @staticmethod
    def from_array(array:typing.Sequence[float], n_agents:int, n_gems:int) -> WorldState:
        ...

    def __deepcopy__(self, _memo:dict) -> WorldState:
        ...

    def __getstate__(self) -> tuple[list[bool], list[tuple[int, int]], list[bool]]:
        ...

    def __setstate__(self, state:tuple[typing.Sequence[bool], typing.Sequence[tuple[int, int]], typing.Sequence[bool]]) -> None:
        ...

    def __getnewargs__(self) -> tuple[list[tuple[int, int]], list[bool]]:
        ...

    def __str__(self) -> str:
        ...

    def __repr__(self) -> str:
        ...

    def __hash__(self) -> int:
        ...

    def __richcmp__(self, other:WorldState, cmp:int) -> bool:
        ...


class Action(Enum):
    r"""
    An action that can be taken in the world by the agents.
    """
    NORTH = auto()
    SOUTH = auto()
    EAST = auto()
    WEST = auto()
    STAY = auto()

    @property
    def delta(self) -> tuple[int, int]:
        """The (i, j) position delta in coordinates for this action."""
        ...
    @property
    def value(self) -> int:
        """The integer value of this action."""
        ...
    @property
    def name(self) -> str:
        """The string name of this action."""
        ...
    ALL: typing.ClassVar[list[Action]]
    """Ordered list of actions"""
    N: typing.ClassVar[int]
    """The number of actions (cardinality of the action space)"""

    def __richcmp__(self, other:Action, op:int) -> bool:
        r"""
        Equality comparison.
        Raises:
            TypeError: If the comparison operator is not == or !=.
        """
        ...

    def __repr__(self) -> str:
        ...

    def opposite(self) -> Action:
        r"""
        The opposite action of this action.
        Note: STAY is its own opposite.
        """
        ...


class EventType(Enum):
    r"""
    An enumeration of the events that can occur in the world.
    """
    AGENT_EXIT = auto()
    GEM_COLLECTED = auto()
    AGENT_DIED = auto()



