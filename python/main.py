from lle.exceptions import InvalidActionError
from lle import World, LLE, Action, EventType
import matplotlib.pyplot as plt
import cv2
from lle.env import SOLLE

str_map = " S0 . G . X\n" + " S1 @ . . .\n" + "L0E . . V V\n" + " @  @ . V V\n" + " G  . . . X"
world = World(str_map)
img = world.get_image()
plt.imshow(img)
plt.show()

import lle

lle.exceptions.InvalidActionError

raise InvalidActionError("Invalid action")
