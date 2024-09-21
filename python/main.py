from lle import WorldState


class ABC(WorldState):
    def __init__(self, agents_positions: list[tuple[int, int]], gems_collected: list[bool], agents_alive: list[bool] | None = None):
        super().__init__(agents_positions, gems_collected=gems_collected, agents_alive=agents_alive)
        self.coins = [1]
        # self.agents_alive = agents_alive
        # self.agents_positions = agents_positions
        # self.gems_collected = gems_collected

    def __hash__(self):
        h = super().__hash__()
        return hash((h, tuple(self.coins)))


x = ABC([(0, 25), (1, 1)], [False, False, True])
y = ABC([(0, 25), (1, 1)], [False, False, True])
print(x, y)
print(hash(x), hash(y))
exit(0)
