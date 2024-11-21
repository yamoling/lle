from lle import LLE, ObservationType
import time

env = LLE.level(1).obs_type("layered").single_objective()
env2 = LLE.level(1).obs_type(ObservationType.LAYERED).single_objective()
assert env.has_same_inouts(env2)
