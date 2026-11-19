"""Reward strategies specific to `CLLE`. The others are those of `lle.env.reward_strategy`."""

from collections.abc import Iterable
from dataclasses import dataclass

from lle.env.reward_strategy import PotentialShaped, RewardStrategy
from lle.types import LaserId, Position

from .world import ContinuousWorld


@dataclass
class PotentialShapedCLLE(PotentialShaped):
    """Potential-based reward shaping that rewards agents for entering the given lasers.

    The cells to reward are the full beam cells of each laser of `lasers_to_reward`, and an agent
    is on the cell that contains its centre (`ContinuousWorld.agents_cells`).

    @ai-generated
    """

    def __init__(
        self,
        strategy: RewardStrategy,
        world: ContinuousWorld,
        gamma: float,
        reward_value: float,
        lasers_to_reward: Iterable[LaserId],
    ):
        """
        Shape `strategy` with the beam cells of the lasers whose ids are `lasers_to_reward`.

        Raises a `ValueError` if a laser id does not exist in `world`.

        @ai-generated
        """
        self._world = world
        super().__init__(
            strategy,
            gamma,
            reward_value,
            self._compute_positions_to_reward(world, lasers_to_reward),
        )

    def get_agents_positions(self):
        """
        The cell of each agent of `self._world`. Reading the world through `self` (rather than a
        closure) keeps `copy.deepcopy` and `pickle` of the environment consistent.

        @ai-generated
        """
        return self._world.agents_cells

    @staticmethod
    def _compute_positions_to_reward(world: ContinuousWorld, lasers_to_reward: Iterable[LaserId]):
        """
        The beam cells of each laser of `lasers_to_reward`.

        @ai-generated
        """
        index_of = {laser_id: index for index, (_, _, _, laser_id) in enumerate(world.laser_sources)}
        beam_cells = world.beam_cells
        pos_to_reward = list[set[Position]]()
        for laser_id in lasers_to_reward:
            if laser_id not in index_of:
                raise ValueError(f"Invalid laser id {laser_id}: the world has the lasers {sorted(index_of)}")
            pos_to_reward.append(set(beam_cells[index_of[laser_id]]))
        return pos_to_reward
