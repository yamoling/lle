from lle import WorldState
from random import choices


class SubWorldState(WorldState):
    def __init__(self, agents_positions: list[tuple[int, int]], gems_collected: list[bool], agents_alive: list[bool], x: int):
        super().__init__(agents_positions, gems_collected, agents_alive)
        self.x = x

    def __new__(cls, agents_positions: list[tuple[int, int]], gems_collected: list[bool], agents_alive: list[bool], *args, **kwargs):
        instance = super().__new__(cls, agents_positions, gems_collected, agents_alive)
        return instance


w = SubWorldState([(1, 1)], [], [True], 1)
print(w)
