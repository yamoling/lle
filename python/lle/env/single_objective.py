from typing import Literal, Optional

from lle import EventType, WorldEvent, World
from lle.observations import ObservationType
from .env import REWARD_DEATH, REWARD_DONE, REWARD_EXIT, REWARD_GEM, LLE


class SOLLE(LLE[float]):
    """
    Single Objective Laser Learning Environment (SOLLE)
    """

    def __init__(
        self,
        world: World,
        obs_type: ObservationType = ObservationType.STATE,
        state_type: ObservationType = ObservationType.STATE,
        death_strategy: Literal["respawn", "end"] = "end",
        walkable_lasers: bool = True,
        name: Optional[str] = None,
    ):
        super().__init__(world, None, obs_type, state_type, name, death_strategy, walkable_lasers)

    def compute_reward(self, events: list[WorldEvent]):
        reward = 0.0
        death_reward = 0.0
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward += REWARD_DEATH
                case EventType.GEM_COLLECTED:
                    reward += REWARD_GEM
                case EventType.AGENT_EXIT:
                    reward += REWARD_EXIT
        if death_reward != 0:
            return death_reward
        if self.n_arrived == self.n_agents:
            reward += REWARD_DONE
        return reward
