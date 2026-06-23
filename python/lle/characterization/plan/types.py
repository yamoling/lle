from typing import Sequence

from lle.world import Action

JointAction = Action | Sequence[Action]
Plan = Sequence[JointAction]
"""A plan is a sequence of joint actions."""
