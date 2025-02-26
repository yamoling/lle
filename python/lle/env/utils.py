from lle import World
from lle import tiles


def get_lasers_of(world: World, source: tiles.LaserSource):
    lasers = list[tiles.Laser]()
    for laser in world.lasers:
        if laser.laser_id == source.laser_id:
            lasers.append(laser)
    return lasers
