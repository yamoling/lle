import cv2

from lle import World


# w = World.from_file("cartes/2_agents/zigzag")

w = World.from_file("level3")


img = w.get_image()

cv2.imshow("Visualisation", img)

# Utilisez waitKey avec 0 pour bloquer et attendre que l'utilisateur appuie sur 'enter' ou avec 1 pour continuer dans le code.

cv2.waitKey(0)  # Attend que l'utilisateur appuie sur 'enter'

cv2.waitKey(1)  # continue l'exécution du code
