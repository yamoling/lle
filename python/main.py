from lle import World, Action, REWARD_END_GAME, REWARD_AGENT_JUST_ARRIVED, REWARD_GEM_COLLECTED


world = World(
    """
    S0 X V .
    .  . . V
    G  . V .
    """
)

import cv2
import time

world.reset()
img = world.get_image()
cv2.imshow("image", img)
cv2.waitKey(1)
time.sleep(0.5)
time.sleep(0.5)
cv2.imshow("image", img)
cv2.waitKey(1)
input()
