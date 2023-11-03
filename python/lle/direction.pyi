from typing import final

@final
class Direction:
    NORTH: Direction
    SOUTH: Direction
    EAST: Direction
    WEST: Direction

    @property
    def name(self) -> str:
        """The string name of this direction."""
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: Direction) -> bool: ...
