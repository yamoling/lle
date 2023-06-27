from typing import Tuple

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
