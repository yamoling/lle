import lle
import cv2
import numpy as np

w = lle.World("../maps/lvl6")
w.reset()
print(w.n_agents)
dims = w.image_dimensions
dims = (dims[1], dims[0])
img = np.array(w.get_image(), dtype=np.uint8).reshape((*dims, 3))
img = cv2.cvtColor(img, cv2.COLOR_RGB2BGR)
cv2.imwrite("lle.png", img)
