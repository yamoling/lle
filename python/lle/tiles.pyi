# This file is automatically generated by pyo3_stub_gen
# ruff: noqa: E501, F401

import typing
from enum import Enum, auto

class Gem:
    is_collected: bool
    """Whether the gem has been collected."""
    pos: tuple[int, int]
    """The (i, j) position of the gem."""
    agent: typing.Optional[int]
    def __str__(self) -> str:
        ...

    def __repr__(self) -> str:
        ...

    def collect(self) -> None:
        ...


class Laser:
    r"""
    A laser tile of the world.
    """
    laser_id: int
    """The ID of the laser (unique per laser source)"""
    agent_id: int
    """The id of the agent that can block the laser."""
    direction: Direction
    """The direction of the laser beam."""
    is_on: bool
    """Whether the laser is turned on."""
    is_enabled: bool
    """Whether the laser is enabled."""
    pos: tuple[int, int]
    """The (i, j) position of the tile."""
    is_off: bool
    """Whether the laser is turned off."""
    is_disabled: bool
    """Whether the laser is disabled."""
    agent: typing.Optional[int]
    """The id of the agent currently standing on the tile, if any."""
    def __str__(self) -> str:
        ...

    def __repr__(self) -> str:
        ...


class LaserSource:
    agent_id: int
    """The id (colour) of the agent that can block the laser."""
    direction: Direction
    """The direction of the laser beam.
    The direction can currently not be changed after creation of the `World`."""
    is_enabled: bool
    """Whether the laser source is enabled."""
    laser_id: int
    """The unique id of the laser."""
    pos: tuple[int, int]
    """The (i, j) position of the laser tile."""
    is_disabled: bool
    """Whether the laser source is disabled."""
    def set_is_enabled(self, enabled:bool) -> None:
        ...

    def set_is_disabled(self, disabled:bool) -> None:
        ...

    def disable(self) -> None:
        r"""
        Disable the laser source and its corresponding laser tiles.
        """
        ...

    def enable(self) -> None:
        r"""
        Enable the laser source and its corresponding laser tiles.
        """
        ...

    def set_agent_id(self, agent_id:int) -> None:
        ...

    def set_colour(self, colour:int) -> None:
        r"""
        Change the colour of the laser to the one of the given agent ID.
        Alias to `source.agent_id = new_agent_id`.
        """
        ...

    def __str__(self) -> str:
        ...

    def __repr__(self) -> str:
        ...


class Direction(Enum):
    NORTH = auto()
    EAST = auto()
    SOUTH = auto()
    WEST = auto()

    @property
    def is_horizontal(self) -> bool:
        ...
    @property
    def is_vertical(self) -> bool:
        ...
    @property
    def name(self) -> str:
        ...

    @staticmethod
    def from_str(direction:str) -> Direction:
        r"""
        Creates a `Direction` from a string representation.
        
        Args:
           direction (Literal["N", "E", "S", "W"]): The string direction to create.
        
        Returns:
          The corresponding `Direction` object.
        
        Raises:
          ValueError: If the string is not a valid cardinal direction.
        """
        ...

    def delta(self) -> tuple[int, int]:
        r"""
        The delta of this direction (di, dj).
        """
        ...

    def opposite(self) -> Direction:
        r"""
        The opposite of this direction.
        """
        ...

    def __repr__(self) -> str:
        ...

    def __getstate__(self) -> str:
        ...

    def __getnewargs__(self) -> typing.Any:
        r"""
        This method is called to instantiate the object before deserialisation.
        It required "default arguments" to be provided to the __new__ method
        before replacing them by the actual values in __setstate__.
        """
        ...

    def __setstate__(self, state:str) -> None:
        ...


