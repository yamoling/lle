import lle
import cv2
import numpy as np

a1 = lle.Action(0)
a2 = lle.Action(0)
res = a1 == a2
print(res)

exit()
env = lle.LLE("map.txt")
env.reset()
env.render("human")
import time

time.sleep(0.2)
env.render("human")
input("Press enter to continue")
exit()
w.reset()
print(w.n_agents)
dims = w.image_dimensions
dims = (dims[1], dims[0])
img = np.array(w.get_image(), dtype=np.uint8).reshape((*dims, 3))
img = cv2.cvtColor(img, cv2.COLOR_RGB2BGR)
cv2.imwrite("lle.png", img)
