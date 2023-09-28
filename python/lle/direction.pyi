from typing import final

@final
class Direction:
    NORTH: "Direction"
    SOUTH: "Direction"
    EAST: "Direction"
    WEST: "Direction"

    @property
    def name(self) -> str:
        """The string name of this direction."""
