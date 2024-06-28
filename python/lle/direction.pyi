from typing import Literal, Tuple, final

@final
class Direction:
    NORTH: Direction
    SOUTH: Direction
    EAST: Direction
    WEST: Direction

    def __init__(self, direction: Literal["N", "S", "E", "W"]):
        """
        Creates a new direction from a string representation.

        Raise a ValueError if the string is not a valid cardinal direction.
        """

    def delta(self) -> Tuple[int, int]:
        """The delta of this direction (di, dj)."""

    def opposite(self) -> Direction:
        """The opposite of this direction."""

    def is_horizontal(self) -> bool: ...
    def is_vertical(self) -> bool: ...
    @property
    def name(self) -> str:
        """The string name of this direction."""
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
