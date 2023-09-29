from .action import Action
from .world import World, WorldState
from .agent import Agent
from .tiles import Gem, Laser, LaserSource, Tile
from .direction import Direction

# Constants
REWARD_AGENT_DIED: float
"""The penalty for dying."""
REWARD_AGENT_JUST_ARRIVED: float
"""The reward for arriving on an exit tile."""
REWARD_END_GAME: float
"""The reward for finishing the game."""
REWARD_GEM_COLLECTED: float
"""The reward for collecting a gem."""
__version__: str
