import cv2
from lle import World, Action, LLE, ObservationType
from rlenv import Observation
import rlenv

env = LLE.level(6, ObservationType.RGB_IMAGE)
obs = env.reset()
env = LLE.from_file("level_6.txt")
LLE.from_str(env.to_str())

env = LLE.level(6, ObservationType.LAYERED)
time_limit = env.width * env.height / 2
env = rlenv.Builder(env).agent_id().time_limit(time_limit).build()
