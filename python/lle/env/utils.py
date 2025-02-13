from lle import World
from lle.tiles import LaserSource, Laser


def get_lasers_of(world: World, source: LaserSource):
    lasers = list[Laser]()
    for laser in world.lasers:
        if laser.laser_id == source.laser_id:
            lasers.append(laser)
    return lasers
