from typing import Tuple


# pylint: disable=no-name-in-module
from .lle import (
    Action,
    World,
    WorldState,
    Agent,
    Gem,
    Laser,
    LaserSource,
    Direction,
    REWARD_AGENT_DIED,
    REWARD_AGENT_JUST_ARRIVED,
    REWARD_END_GAME,
    REWARD_GEM_COLLECTED,
)
from .env import LLE
from .observations import ObservationType

Position = Tuple[int, int]
AgentId = int
