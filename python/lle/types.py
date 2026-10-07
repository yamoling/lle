"""Shared semantic aliases used across the Python API.

These aliases document intent rather than introduce new runtime behaviour.
Use them to make function signatures and examples easier to read.
"""

Position = tuple[int, int, int]
"""
Represents a position (i, j, k) in the gridworld.

This is a semantic type wrapper around tuple[int, int, int].
"""

AgentId = int
"""
The integer identifier of an agent.
"""

Colour = int
"""
The colour of an agent, a laser beam, or a lift/button authorization.

Distinct from `AgentId`: several agents may share one colour, and a colour is
what decides which beams an agent may block and cross and which lifts and
buttons it may use.
"""

LaserId = int
"""
The identifier of a laser source.

The beam carries the same identifier as its source.
"""
