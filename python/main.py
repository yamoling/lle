from lle import World

content = '''
width = 10
height = 5
map_string = """
. . . . S1 . . . . . 
. . . . .  . . . . . 
. . . . .  . . . . . 
. . . . .  . . . . . 
. . . . .  . . . . . 
"""

[[agents]]
start_positions = { rectangles = [{ i_min = 0, i_max = 10 }] }

[[agents]]
# Deduced from the string map that agent 1 has a start position at (0, 5).

[[agents]]
start_positions = { positions = [{ i = 0, j = 5 }, { i = 10, j = 5 }] }

[[agents]]
start_positions = { positions = [
    { i = 5, j = 10 },
], rectangles = [
    { i_min = 1, imax = 3, j_min = 0, j_max = 3 },
    { j_min = 4 },
] }
'''
world = World(content)
assert world.width == 10
assert world.height == 5
assert world.n_agents == 4
