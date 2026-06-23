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
    _solver: solver.Solver = field(init=False, repr=False)

    def __post_init__(self) -> None:
        self._solver = solver.Solver(self.world, self.t_max)

    @cached_property
    def n_laser_colours(self) -> int:
        """Number of distinct agent colours that own a laser source."""
        return len({source.agent_id for source in self.world.laser_sources})

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

    def is_solvable(self):
        return self.shortest_path is not None

    def is_independent(self):
        """
        # Raises
            -`NotSolvableError` if the world is not solvable
        """
        if not self.is_solvable:
            raise NotSolvableError("World is not solvable.")
        return self.shortest_independent_path is not None

    def is_asymmetric(self):
        """
        Whether the world requires asymmetric cooperation:
        - the world is solvable,
        - at least one solution exhibits a help edge whose helper is never helped, and
        - no solution within `t_max` avoids all such asymmetric help edges.

        # Raises
            -`NotSolvableError` if the world is not solvable
        """
        path = self.shortest_path
        if path is None:
            raise NotSolvableError("Cannot determine if requires asymmetric cooperation if unsolvable.")
        if self.n_laser_colours == 0:
            return False
        profile = profile_trajectory(self.world, path)
        if not profile.is_asymmetric:
            return False
        if self.shortest_independent_path is not None:
            return False
        return self.shortest_non_asymmetric_path is None

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

        A mutual cycle `a → b → a` is a chain of length 2 even when the two help events occur
        simultaneously, because chained cooperation uses non-decreasing timestamps.

        # Raises
            -`NotSolvableError` if the world is not solvable
        """
        self._validate_dependency_order(length)
        path = self.shortest_path
        if path is None:
            raise NotSolvableError("World is not solvable")
        if self.shortest_independent_path is not None:
            return False
        cached = self._infer_from_monotone_cache(self._no_chain_cache, length)
        if cached is not None:
            return cached
        profile = profile_trajectory(self.world, path)
        if not profile.is_chained(length):
            return False
        is_chained = self.shortest_non_chained_path(length) is None
        self._propagate_monotone_cache(self._no_chain_cache, length)
        return is_chained

    def is_interdependent(self, n_agents: int = 2) -> bool:
        """
        Whether the world *requires* interdependence between at least `n_agents` agents:
        - the optimal trajectory's dependency graph contains a temporal cycle of order >= `n_agents`, and
        - no solution within`t_max` avoids all such cycles.

        For two agents this coincides with`is_mutual`: a cycle of order 2 is exactly a mutual
        `a → b → a` under the non-strict temporal-cycle semantics used by interdependence.

        # Raises
            -`NotSolvableError` if the world is not solvable
        """
        self._validate_dependency_order(n_agents)
        path = self.shortest_path
        if path is None:
            raise NotSolvableError("World is not solvable")
        if self.shortest_independent_path is not None:
            return False
        if n_agents > self.n_laser_colours:
            self._no_interdependence_cache[n_agents] = path
            return False
        cached = self._infer_from_monotone_cache(self._no_interdependence_cache, n_agents)
        if cached is not None:
            return cached
        profile = profile_trajectory(self.world, path)
        if not profile.is_interdependent(n_agents):
            return False
        is_interdependent = self.shortest_non_interdependent_path(n_agents) is None
        self._propagate_monotone_cache(
            self._no_interdependence_cache,
            n_agents,
            upper_bound=self.n_laser_colours,
        )
        return is_interdependent

    @cached_property
    def shortest_path(self):
        return self._solver.solve()

    @cached_property
    def shortest_independent_path(self):
        """The length of the shortest valid plan within [lower_bound, t_max] that does not involve cooperation, or None if unsolvable."""
        return self._solver.solve("no-cooperation")

    @cached_property
    def shortest_non_asymmetric_path(self):
        if self.n_laser_colours == 0:
            return self.shortest_path
        if "shortest_independent_path" in self.__dict__ and self.shortest_independent_path is not None:
            return self.shortest_independent_path
        return self._solver.solve("no-asymmetric")

    @cached_property
    def shortest_non_mutual_path(self):
        return self._solver.solve("no-mutual")

    def shortest_non_chained_path(self, length: int):
        """Shortest plan within `t_max` that avoids every chain of length >= `length`, or None."""
        self._validate_dependency_order(length)
        if "shortest_independent_path" in self.__dict__ and self.shortest_independent_path is not None:
            self._no_chain_cache[length] = self.shortest_independent_path
        is_known, inferred = self._infer_no_dependency_result(self._no_chain_cache, length)
        if is_known:
            self._no_chain_cache[length] = inferred
            return inferred
        if length not in self._no_chain_cache:
            self._no_chain_cache[length] = self._solver.solve(f"no-chain-{length}")
            self._propagate_monotone_cache(self._no_chain_cache, length)
        return self._no_chain_cache[length]

    def shortest_non_interdependent_path(self, order: int):
        """Shortest plan within `t_max` that avoids every cycle of order >= `order`, or None."""
        self._validate_dependency_order(order)
        if order > self.n_laser_colours:
            self._no_interdependence_cache[order] = self.shortest_path
        elif "shortest_independent_path" in self.__dict__ and self.shortest_independent_path is not None:
            self._no_interdependence_cache[order] = self.shortest_independent_path
        is_known, inferred = self._infer_no_dependency_result(self._no_interdependence_cache, order)
        if is_known:
            self._no_interdependence_cache[order] = inferred
            return inferred
        if order not in self._no_interdependence_cache:
            self._no_interdependence_cache[order] = self._solver.solve(f"no-interdependence-{order}")
            self._propagate_monotone_cache(
                self._no_interdependence_cache,
                order,
                upper_bound=self.n_laser_colours,
            )
        return self._no_interdependence_cache[order]

    @staticmethod
    def _validate_dependency_order(order: int) -> None:
        if order < 2:
            raise ValueError(f"Dependency order must be >= 2, got {order}.")

    def _infer_from_monotone_cache(self, cache: dict[int, list | None], order: int) -> bool | None:
        is_known, inferred = self._infer_no_dependency_result(cache, order)
        if is_known:
            cache[order] = inferred
            return inferred is None
        return None

    def _infer_no_dependency_result(self, cache: dict[int, list | None], order: int):
        for cached_order, path in cache.items():
            if cached_order <= order and path is not None:
                return True, path
        if any(cached_order >= order and path is None for cached_order, path in cache.items()):
            return True, None
        return False, None

    def _propagate_monotone_cache(
        self,
        cache: dict[int, list | None],
        order: int,
        *,
        upper_bound: int | None = None,
    ) -> None:
        path = cache[order]
        if path is None:
            for lower_order in range(2, order):
                cache.setdefault(lower_order, None)
        elif upper_bound is not None:
            for higher_order in range(order + 1, upper_bound + 1):
                cache.setdefault(higher_order, path)
