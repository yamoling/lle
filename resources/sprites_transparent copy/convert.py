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
    img = cv2.imread(path)
    img = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
    cv2.imwrite(path, img)
