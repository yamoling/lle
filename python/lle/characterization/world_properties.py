from dataclasses import dataclass
from typing import Literal

from ..world import Action


@dataclass
class WorldProperties:
    shortest_path_length: int | None | Literal["not-computed"] = "not-computed"
    shortest_path_example: list[tuple[Action, ...]] | None | Literal["not-computed"] = "not-computed"
    shortest_independent_path_length: int | None | Literal["not-computed"] = "not-computed"
    shortest_independent_path_example: list[tuple[Action, ...]] | None | Literal["not-computed"] = "not-computed"
