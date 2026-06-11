from typing import Sequence

from lle.world import Action

JointAction = Action | Sequence[Action]
Trajectory = Sequence[JointAction]
