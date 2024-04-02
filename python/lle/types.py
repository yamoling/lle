Position = tuple[int, int]
"""
Represents a position (i, j) in the gridworld.

This is a semantic type wrapper around tuple[int, int].
"""

AgentId = int
"""
The integer identifier of an agent.

This is a semantic type wrapper around int.
"""

LaserId = int
"""
The positive integer identifier of a laser.

There is one ID for each laser source. The laser beam has the same ID as the source.
"""
