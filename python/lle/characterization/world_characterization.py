from functools import cached_property
from typing import final

from typing_extensions import override

from lle.solver.clauses import SolveMode

from .. import solver
from ..world import World
from .monotone_cache import MonotoneCache
from .plan import profile_plan


@final
class WorldCharacterizer:
    """
    Lazy world characterizer class. Computes the properties of the world on-demand.

    # Note
    All properties are `t_max` dependent, i.e. a world can be said to be cooperative with t_max=10, but this same
    world may be independent for t_max=11.
    """

    def __init__(self, world: World, t_max: int) -> None:
        self.world = world
        self.t_max = t_max
        self._solver = solver.Solver(self.world, self.t_max)
        self._is_chained_cache = MonotoneCache()
        self._is_convergent_cache = MonotoneCache()
        # Convergence and divergence are independent predicates over opposite endpoints of the
        # same directed relation, so they must not share a cache.
        self._is_divergent_cache = MonotoneCache()
        self._interdependence_cache = dict[int, bool]()

    @cached_property
    def n_laser_colours(self) -> int:
        """Number of distinct agent colours that own a laser source."""
        return len({source.agent_id for source in self.world.laser_sources})

    def is_cooperative(self):
        if not self.is_solvable():
            return False
        return self.shortest_independent_path is None

    def is_solvable(self):
        return self.shortest_path is not None

    def is_independent(self):
        if not self.is_solvable():
            return False
        return self.shortest_independent_path is not None

    def is_asymmetric(self):
        """
        Whether the world requires asymmetric cooperation:
        - the world is solvable,
        - at least one solution exhibits a help edge whose helper is never helped, and
        - no solution within `t_max` avoids all such asymmetric help edges.
        """
        path = self.shortest_path
        if path is None:
            return False
        if self.n_laser_colours == 0:
            return False
        profile = profile_plan(self.world, path)
        if not profile.is_asymmetric:
            return False
        if self.shortest_independent_path is not None:
            return False
        return self.shortest_non_asymmetric_path is None

    def is_fully_coupled(self):
        """Every agent helps every other agent at some point."""
        raise NotImplementedError()

    def is_chained(self, length: int = 2) -> bool:
        """
        Whether the world requires chained cooperation of at least `length` help edges:
        - The world is solvable
        - and the optimal trajectory exhibits a chain of length >= `length`
          (e.g. `length=2`: a helped b, then b helped c)
        - and no trajectory within `t_max` avoids every chain of length >= `length`.

        A mutual cycle `a → b → a` is a chain of length 2 even when the two help events occur
        simultaneously, because chained cooperation uses non-decreasing timestamps.
        """
        if length < 2:
            raise ValueError(f"Chain length must be >= 2, got {length}.")
        cached = self._is_chained_cache.get(length)
        if cached is not None:
            return cached
        if self.shortest_path is None:
            return False
        profile = profile_plan(self.world, self.shortest_path)
        if not profile.is_chained(length):
            res = False
        else:
            res = self.compute_shortest_path_without_chain(length) is None
        self._is_chained_cache[length] = res
        return res

    def is_convergent(self, k: int = 2) -> bool:
        """Whether every solution requires one beneficiary to have `k` distinct helpers.

        A shortest normal plan first provides a concrete positive witness. If that plan is
        k-convergent, a no-convergence solve proves universality by returning `None`.

        # Raises
            - `ValueError` if `k < 2`.
        """
        if k < 2:
            raise ValueError(f"Convergence requires at least 2 distinct helpers, got {k}.")
        cached = self._is_convergent_cache.get(k)
        if cached is not None:
            return cached
        path = self.shortest_path
        if path is None:
            return False
        if not profile_plan(self.world, path).is_convergent(k):
            res = False
        else:
            res = self.compute_shortest_path_without_convergence(k) is None
        self._is_convergent_cache[k] = res
        return res

    def is_divergent(self, k: int = 2) -> bool:
        """Whether every solution requires one helper to have `k` distinct beneficiaries.

        This is the outgoing dual of `is_convergent`. A shortest normal plan first provides a
        concrete positive witness. If that plan is k-divergent, a no-divergence solve proves
        universality by returning `None`.

        # Raises
            - `ValueError` if `k < 2`.
        """
        if k < 2:
            raise ValueError(f"Divergence requires at least 2 distinct beneficiaries, got {k}.")
        if k >= self.world.n_agents:
            return False
        cached = self._is_divergent_cache.get(k)
        if cached is not None:
            return cached
        path = self.shortest_path
        if path is None:
            return False
        if not profile_plan(self.world, path).is_divergent(k):
            res = False
        else:
            res = self.compute_shortest_path_without_divergence(k) is None
        self._is_divergent_cache[k] = res
        return res

    def is_interdependent(self, n_agents: int = 2) -> bool:
        """
        Whether the world *requires* interdependence between exactly `n_agents` agents:
        - the optimal trajectory contains a temporal closed trail with exactly that support, and
        - no solution within `t_max` avoids every closed trail of exactly that order.

        Exact orders are not monotone: an order-4 trail does not imply an
        order-3 trail. For two agents this coincides with `is_mutual`.

        # Raises
            -`NotSolvableError` if the world is not solvable
        """
        if n_agents < 2:
            raise ValueError(f"Interdependence only makes sense for >= 2 agents. Got {n_agents}.")
        cached = self._interdependence_cache.get(n_agents)
        if cached is not None:
            return cached
        if self.shortest_path is None:
            return False
        profile = profile_plan(self.world, self.shortest_path)
        if not profile.is_interdependent(n_agents):
            res = False
        else:
            res = self.compute_shortest_non_interdependent_path(n_agents) is None
        self._interdependence_cache[n_agents] = res
        return res

    @cached_property
    def shortest_path(self):
        return self._solver.find_shortest(SolveMode.standard())

    @cached_property
    def shortest_independent_path(self):
        """The shortest valid plan within `t_max` that does not involve cooperation, or `None`."""
        return self._solver.find_shortest(SolveMode.no_cooperation())

    @cached_property
    def shortest_non_asymmetric_path(self):
        return self._solver.find_shortest(SolveMode.no_asymmetric())

    @cached_property
    def shortest_non_mutual_path(self):
        return self._solver.find_shortest(SolveMode.no_mutual())

    def compute_shortest_path_without_chain(self, length: int):
        if length < 2:
            raise ValueError(f"Chain length must be >= 2, got {length}.")
        return self._solver.find_shortest(SolveMode.no_chain(length))

    def compute_shortest_path_without_convergence(self, k: int):
        """Return the shortest plan that avoids k-convergence, or `None` if none exists."""
        if k < 2:
            raise ValueError(f"Convergence requires at least 2 distinct helpers, got {k}.")
        return self._solver.find_shortest(SolveMode.no_convergence(k))

    def compute_shortest_path_without_divergence(self, k: int):
        """Return the shortest plan that avoids k-divergence, or `None` if none exists."""
        if k < 2:
            raise ValueError(f"Divergence requires at least 2 distinct beneficiaries, got {k}.")
        return self._solver.find_shortest(SolveMode.no_divergence(k))

    def compute_shortest_non_interdependent_path(self, order: int):
        """Shortest plan within `t_max` that avoids every cycle of exactly `order`, or None."""
        if order < 2:
            raise ValueError(f"Interdependence order must be >= 2, got {order}.")
        return self._solver.find_shortest(SolveMode.no_interdependence(order))

    @override
    def __eq__(self, value: object, /):
        if not isinstance(value, WorldCharacterizer):
            return False
        return self.world == value.world and self.t_max == value.t_max

    @override
    def __hash__(self) -> int:
        return hash((self.world, self.t_max))
