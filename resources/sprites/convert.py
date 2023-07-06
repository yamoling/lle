import cv2
import os

images = ["gem.png"]
for file in os.listdir("lasers/horizontal"):
    images.append("lasers/horizontal/" + file)
for file in os.listdir("lasers/vertical"):
    images.append("lasers/vertical/" + file)
for direction in ["north", "south", "west", "east"]:
    for file in os.listdir(f"laser_sources/{direction}"):
        images.append(f"laser_sources/{direction}/{file}")
for file in os.listdir("agents"):
    images.append(f"agents/{file}")

for path in images:
    img = cv2.imread(path, cv2.IMREAD_UNCHANGED)
    img = cv2.cvtColor(img, cv2.COLOR_BGRA2RGBA)
    cv2.imwrite(path, img)
