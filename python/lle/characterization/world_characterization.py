from dataclasses import dataclass
from functools import cached_property

from .. import solver
from ..world import World
from .trajectory import profile_trajectory, TrajectoryProfile


class NotSolvableError(ValueError):
    """Raised when a world is not solvable."""


@dataclass
class WorldCharacterizer:
    """
    Lazy world characterizer class. Computes the properties of the world on-demand.

    # Note
    All properties are `t_max` dependent, i.e. a world can be said to be cooperative with t_max=10, but this same
    world may be independent for t_max=11.
    """

    world: World
    t_max: int

    @property
    def is_cooperative(self):
        """
        # Returns
        Returns whether the world is cooperative.

        # Raises
            - ``NotSolvableError`` if the world is not solvable
        """
        if not self.is_solvable:
            raise NotSolvableError("World is not solvable")
        return self.shortest_independent_path is None

    @property
    def is_solvable(self):
        return self.shortest_path is not None

    @property
    def is_independent(self):
        """
        # Raises
            - ``NotSolvableError`` if the world is not solvable
        """
        if not self.is_solvable:
            raise NotSolvableError("World is not solvable.")
        return self.shortest_independent_path is not None

    @property
    def is_mutual(self):
        """
        - The world is solvable
        - and there exists a mutual trajectory
        - and the world would be unsolvable without mutual help

        # Raises
            - ``NotSolvableError`` if the world is not solvable
        """
        path = self.shortest_path
        if path is None:
            raise NotSolvableError("Cannot determine if requires mutual cooperation if unsolvable.")
        profile = profile_trajectory(self.world, path)
        # If the trajectory is not even mutual, then it cannot require mutual cooperation.
        if not profile.is_mutual:
            return False
        # If there does not exist a non-mutual trajectory, then the world requires mutual cooperation.
        return self.shortest_non_mutual_path is None

    @property
    def is_chained(self):
        """
        Whether the world requires chained cooperation:
        - The world is solvable
        - and the optimal trajectory exhibits a chain (a helped b, then b helped c)
        - and no non-chained trajectory exists within `t_max`

        Chained cooperation subsumes mutual cooperation: a mutual cycle `a → b → a` is a
        chain of length 2.

        # Raises
            - ``NotSolvableError`` if the world is not solvable
        """
        path = self.shortest_path
        if path is None:
            raise NotSolvableError("World is not solvable")
        profile = profile_trajectory(self.world, path)
        if not profile.is_chained:
            return False
        return self.shortest_non_chained_path is None

    def is_interdependent(self, k: int = 2) -> bool:
        """
        Whether the world *requires* interdependence at level ``k``:
        - the optimal trajectory exhibits a temporal cycle of order ≥ ``k``, and
        - no solution within ``t_max`` avoids all such cycles.

        # Raises
            - ``NotSolvableError`` if the world is not solvable
        """
        path = self.shortest_path
        if path is None:
            raise NotSolvableError("World is not solvable")
        profile = profile_trajectory(self.world, path)
        if not profile.is_interdependent(k):
            return False
        return self._shortest_non_interdependent_path(k) is None

    def _shortest_non_interdependent_path(self, k: int):
        return solver.solve(self.world, self.t_max, mode=f"no-interdependence-{k}")

    @cached_property
    def shortest_path(self):
        return solver.solve(self.world, self.t_max)

    @cached_property
    def shortest_independent_path(self):
        """The length of the shortest valid plan within [lower_bound, t_max] that does not involve cooperation, or None if unsolvable."""
        return solver.solve(self.world, self.t_max, mode="no-cooperation")

    @cached_property
    def shortest_non_mutual_path(self):
        return solver.solve(self.world, self.t_max, mode="no-mutual-cooperation")

    @cached_property
    def shortest_non_chained_path(self):
        return solver.solve(self.world, self.t_max, mode="no-chained-cooperation")
