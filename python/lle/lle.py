from .action import Action
from .world import World, WorldState
from .agent import Agent
from .tile import Gem, Laser, LaserSource, Tile
from .direction import Direction
from .event import WorldEvent, EventType

# Constants
REWARD_AGENT_DIED: float
"""The penalty for dying."""
REWARD_AGENT_EXIT: float
"""The reward for arriving on an exit tile."""
REWARD_END_GAME: float
"""The reward for finishing the game."""
REWARD_GEM_COLLECTED: float
"""The reward for collecting a gem."""
__version__: str
