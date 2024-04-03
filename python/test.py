import random
from numpy import ndarray
from rlenv.wrappers import RLEnvWrapper
from lle import LLE, LaserSource
from dataclasses import dataclass
from serde import serde
import lle
import time
import matplotlib.pyplot as plt


@serde
@dataclass
class LaserCurriculum(RLEnvWrapper):
    def __init__(self, env: LLE):
        super().__init__(env)
        self.world = env.world
        self.top_left_laser = self.world.laser_sources[0, 1]
        self.top_laser = self.world.laser_sources[4, 0]
        self.bot_laser = self.world.laser_sources[6, 12]
        self.t = 0

    def randomize(self, source: LaserSource, p_enabled: float, p_colour: float):
        if random.random() <= p_enabled:
            self.world.enable_laser_source(source)
            if random.random() <= p_colour:
                colour = random.randint(0, self.n_agents - 1)
                self.world.set_laser_colour(source, colour)
        else:
            self.world.disable_laser_source(source)

    def step(self, actions: list[int] | ndarray):
        self.t += 1
        return super().step(actions)

    def reset(self):
        """
        - < 100k: disable all lasers
        - < 200k: enable bottom laser with 50% probability
        - < 300k: enable bottom laser with 50% probability with a random colour
        - < 400k: enable bot + top laser with 50% probability. Top laser has a random colour.
        - < 500k: enable bot + top laser with 50% probability. Both lasers have a random colour.
        - < 600k: enable all lasers with random colours.
        - < 700k: enable all lasers with original colours.
        """
        if self.t < 100_000:
            self.world.disable_laser_source(self.top_left_laser)
            self.world.disable_laser_source(self.top_laser)
            self.world.disable_laser_source(self.bot_laser)
        elif self.t < 200_000:
            self.randomize(self.bot_laser, 0.5, 0.0)
        elif self.t < 300_000:
            self.randomize(self.bot_laser, 0.5, 1.0)
        elif self.t < 400_000:
            self.randomize(self.bot_laser, 0.5, 1.0)
            self.randomize(self.top_laser, 0.5, 0.0)
        elif self.t < 500_000:
            self.randomize(self.bot_laser, 0.5, 1.0)
            self.randomize(self.top_laser, 0.5, 1.0)
        elif self.t < 600_000:
            self.randomize(self.bot_laser, 1.0, 1.0)
            self.randomize(self.top_laser, 1.0, 1.0)
            self.randomize(self.top_left_laser, 1.0, 1.0)
        elif self.t < 700_000:
            self.world.enable_laser_source(self.top_left_laser)
            self.world.enable_laser_source(self.bot_laser)
            self.world.enable_laser_source(self.top_laser)
            self.world.set_laser_colour(self.top_laser, 0)
            self.world.set_laser_colour(self.bot_laser, 1)
            self.world.set_laser_colour(self.top_left_laser, 2)

        return super().reset()


def show(world: lle.World):
    world.reset()
    img = world.get_image()
    plt.imshow(img)
    plt.show(block=False)
    input("Press Enter to continue...")


world = lle.World.level(6)
print(world.laser_sources)
show(world)

world.disable_laser_source(world.laser_sources[0, 1])
show(world)


world.disable_laser_source(world.laser_sources[4, 0])
show(world)
world.disable_laser_source(world.laser_sources[6, 12])
show(world)
