from typing import List, Tuple, final

@final
class Action:
    """Enumeration of all possible actions."""

    NORTH: "Action"
    SOUTH: "Action"
    EAST: "Action"
    WEST: "Action"
    STAY: "Action"

    N: int
    """Enum cardinality"""

    ALL: List["Action"]
    """List of all variants"""

    def __init__(self, action_num: int):
        """Create an action from an integer identifier. Invalid values (< 0 or > 5) raise a ValueError."""
    @property
    def delta(self) -> Tuple[int, int]:
        """The delta (i, j) in coordinates for this action."""
    @property
    def value(self) -> int:
        """The integer value of this action."""
    @property
    def name(self) -> str:
        """The string name of this action."""
