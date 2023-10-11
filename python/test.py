import lle
from lle import Action


w = lle.World(
    """
    S0 . G
    S1 X X
"""
)
w.step([Action.EAST, Action.STAY])
