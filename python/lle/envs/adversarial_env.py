from typing import Any
import numpy as np
from rlenv import RLEnv, Observation
from rlenv.models.spaces import MultiDiscreteActionSpace, MultiDiscreteSpace, DiscreteSpace
from lle import WorldBuilder, Direction


WALL_IDX = 0
GEM_IDX = 1
EXIT_IDX = 2
AGENT_0_IDX = 3


START = 0
EXIT = 1
WALL = 2
GEM = 3
LASER_NORTH = 4
LASER_EAST = 5
LASER_SOUTH = 6
LASER_WEST = 7
STOP = 8


class AdversarialEnvLLE(RLEnv[MultiDiscreteActionSpace]):
    def __init__(self, width: int, height: int, n_agents: int):
        self.builder = WorldBuilder(width, height, n_agents)
        labels = ["Start", "Exit", "Wall", "Gem", "Laser North", "Laser East", "Laser South", "Laser West", "STOP"]
        self.tile_action_space = DiscreteSpace(len(labels), labels=labels)
        self.index_action_space = DiscreteSpace(height * width, [f"({i}, {j})" for i in range(height) for j in range(width)])
        self.agent_action_space = DiscreteSpace(n_agents, labels=[f"Agent {i}" for i in range(n_agents)])

        action_space = MultiDiscreteActionSpace(
            1,
            MultiDiscreteSpace(
                self.tile_action_space, self.index_action_space, self.agent_action_space, labels=["Tile type", "Index", "Agent"]
            ),
        )
        observation_shape = (self.builder.height, self.builder.width, AGENT_0_IDX + 2 * n_agents)
        state_shape = observation_shape
        super().__init__(
            action_space,
            observation_shape,
            state_shape,
            (0,),
            DiscreteSpace(1, ["Reward"]),
        )
        self.LASER_IDX_0 = AGENT_0_IDX + n_agents
        self.observation = np.zeros((self.builder.height, self.builder.width, self.LASER_IDX_0 + self.n_agents), dtype=np.float32)

    def get_state(self):
        raise NotImplementedError()

    def available_actions(self) -> list[np.ndarray[bool, Any]]:
        def available_tile_type():
            """
            If
                1. If there remain agents without start positions, force to place start positions
                2. If there are not enough exit tiles, force to place exit tiles
                3. Otherwise, all actions are available
                    - except 'START'
                    - 'STOP' action is only available if the builder can indeed build the world
            """
            if len(self.builder.start_positions) < self.n_agents:
                res = np.full((self.tile_action_space.size,), False, dtype=bool)
                res[START] = True
                return res
            if len(self.builder.exit_positions) < self.n_agents:
                res = np.full((self.tile_action_space.size,), False, dtype=bool)
                res[EXIT] = True
                return res

            res = np.full((self.tile_action_space.size,), True, dtype=bool)
            res[START] = False
            res[STOP] = self.builder.can_build()
            return res

        def available_index():
            res = np.full((self.builder.height * self.builder.width,), False, dtype=bool)
            for i, j in self.builder.available_positions:
                res[i * self.builder.width + j] = True
            return res

        return [available_tile_type(), available_index(), np.full((self.n_agents,), True, dtype=bool)]

    def step(self, actions: np.ndarray):
        index, tile, agent = actions[0]
        i = index // self.builder.width
        j = index % self.builder.width
        done = False
        if tile == START:
            self.builder.set_start((i, j), agent)
            layer = AGENT_0_IDX + agent
            self.observation[layer, i, j] = 1
        elif tile == EXIT:
            self.builder.add_exit((i, j))
            self.observation[EXIT_IDX, i, j] = 1
        elif tile == WALL:
            self.builder.add_wall((i, j))
            self.observation[WALL_IDX, i, j] = 1
        elif tile == GEM:
            self.builder.add_gem((i, j))
            self.observation[GEM_IDX, i, j] = 1
        elif tile == STOP:
            done = True
        else:
            if tile == LASER_NORTH:
                direction = Direction.NORTH
            elif tile == LASER_EAST:
                direction = Direction.EAST
            elif tile == LASER_SOUTH:
                direction = Direction.SOUTH
            elif tile == LASER_WEST:
                direction = Direction.WEST
            self.builder.add_laser_source((i, j), agent, direction)
            layer = self.LASER_IDX_0 + agent
            self.observation[WALL, i, j] = 1
            self.observation[layer, i, j] = 1

        obs = Observation(
            self.observation,
            self.available_actions(),  # type: ignore
            self.get_state(),
        )
        return obs, 0, done, False, {}

    def reset(self):
        self.builder.reset()
        self.observation = np.zeros((self.builder.height, self.builder.width, self.LASER_IDX_0 + self.n_agents), dtype=np.float32)

    def render(self, mode="human"):
        print(self.builder.world_str())
