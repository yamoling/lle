from dataclasses import dataclass, field
from functools import cached_property

from .. import solver
from ..world import World
from .trajectory import profile_trajectory


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
    _no_chain_cache: dict[int, list | None] = field(default_factory=dict, init=False, repr=False)
    _no_interdependence_cache: dict[int, list | None] = field(default_factory=dict, init=False, repr=False)

    @property
    def is_cooperative(self):
        """
        # Returns
        Returns whether the world is cooperative.

        # Raises
            -`NotSolvableError` if the world is not solvable
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
            -`NotSolvableError` if the world is not solvable
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
            -`NotSolvableError` if the world is not solvable
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

    def is_chained(self, length: int = 2) -> bool:
        """
        Whether the world requires chained cooperation of at least `length` help edges:
        - The world is solvable
        - and the optimal trajectory exhibits a chain of length >= `length`
          (e.g. `length=2`: a helped b, then b helped c)
        - and no trajectory within `t_max` avoids every chain of length >= `length`.

        Chained cooperation subsumes mutual cooperation: a mutual cycle `a → b → a` is a
        chain of length 2, so for a two-agent world`is_chained(2)` matches`is_mutual`.

        # Raises
            -`NotSolvableError` if the world is not solvable
        """
        path = self.shortest_path
        if path is None:
            raise NotSolvableError("World is not solvable")
        profile = profile_trajectory(self.world, path)
        if not profile.is_chained(length):
            return False
        return self.shortest_non_chained_path(length) is None

    def is_interdependent(self, n_agents: int = 2) -> bool:
        """
        Whether the world *requires* interdependence between at least `n_agents` agents:
        - the optimal trajectory's dependency graph contains a temporal cycle of order >= `n_agents`, and
        - no solution within`t_max` avoids all such cycles.

        For two agents this coincides with`is_mutual` (and with`is_chained(2)`): a cycle of
        order 2 is exactly a mutual `a → b → a`.

        # Raises
            -`NotSolvableError` if the world is not solvable
        """
        path = self.shortest_path
        if path is None:
            raise NotSolvableError("World is not solvable")
        profile = profile_trajectory(self.world, path)
        if not profile.is_interdependent(n_agents):
            return False
        # A temporal cycle of order `n` is also a chain of length `n`, so requiring an order-`n`
        # cycle entails requiring a length-`n` chain. If chains of length `n` are avoidable, the
        # cycles are too: short-circuit without the (stricter) interdependence solve.
        if not self.is_chained(n_agents):
            return False
        return self.shortest_non_interdependent_path(n_agents) is None

    @cached_property
    def shortest_path(self):
        return solver.solve(self.world, self.t_max)

    @cached_property
    def shortest_independent_path(self):
        """The length of the shortest valid plan within [lower_bound, t_max] that does not involve cooperation, or None if unsolvable."""
        return solver.solve(self.world, self.t_max, mode="no-cooperation")

    @cached_property
    def shortest_non_mutual_path(self):
        return solver.solve(self.world, self.t_max, mode="no-mutual")

    def shortest_non_chained_path(self, length: int):
        """Shortest plan within `t_max` that avoids every chain of length >= `length`, or None."""
        if length not in self._no_chain_cache:
            self._no_chain_cache[length] = solver.solve(self.world, self.t_max, mode=f"no-chain-{length}")
        return self._no_chain_cache[length]

    def shortest_non_interdependent_path(self, order: int):
        """Shortest plan within `t_max` that avoids every cycle of order >= `order`, or None."""
        if order not in self._no_interdependence_cache:
            self._no_interdependence_cache[order] = solver.solve(self.world, self.t_max, mode=f"no-interdependence-{order}")
        return self._no_interdependence_cache[order]
