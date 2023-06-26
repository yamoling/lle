import lle


w = lle.World("../maps/lvl1")
w.reset()
img = w.get_image()
print(w.n_agents)
