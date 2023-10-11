import lle
import pickle
from lle import Action


w = lle.World(
    """
    S0 . G
    S1 X X
"""
)
w.reset()
w.step([Action.EAST, Action.EAST])
s = w.get_state()
print(s)


with open("test.pkl", "wb") as f:
    pickle.dump(s, f)


with open("test.pkl", "rb") as f:
    s2 = pickle.load(f)

assert s == s2
print(s2)


with open("test.pkl", "wb") as f:
    print(w.world_string)
    pickle.dump(w, f)

with open("test.pkl", "rb") as f:
    w: lle.World = pickle.load(f)
    print(w.world_string)
    print(w.agents_positions)
