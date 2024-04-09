from typing import List, Tuple, final

@final
class Action:
    """An action in the World."""

    NORTH: "Action"
    SOUTH: "Action"
    EAST: "Action"
    WEST: "Action"
    STAY: "Action"

    N: int
    """The number of actions"""

    ALL: List["Action"]
    """Ordered list of all actions"""

    def __init__(self, action_num: int):
        """Create an action from an integer identifier. Invalid values (< 0 or > 4) raise a `ValueError`."""

    def opposite(self) -> Action:
        """Return the opposite action."""

    @property
    def delta(self) -> Tuple[int, int]:
        """The (i, j) position delta in coordinates for this action."""
    @property
    def value(self) -> int:
        """The integer value of this action."""
    @property
    def name(self) -> str:
        """The string name of this action."""
