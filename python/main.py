from lle import LLE, ObservationType
from lle import exceptions
from lle.tiles import Laser
from lle import tiles
import time


e = exceptions.InvalidActionError()
if isinstance(e, tiles.Laser):
    print("")
env = LLE.level(1).obs_type("layered").build()
env2 = LLE.level(1).obs_type(ObservationType.LAYERED).build()
assert env.has_same_inouts(env2)
