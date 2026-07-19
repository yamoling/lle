from .graph import TemporalCooperationGraph


class PlanProfile:
    def __init__(self, graph: TemporalCooperationGraph) -> None:
        self.graph = graph

    @property
    def is_independent(self):
        return self.graph.is_empty

    @property
    def is_cooperative(self):
        return not self.graph.is_empty

    @property
    def is_asymmetric(self):
        """Whether the trajectory has a help edge whose helper is never helped."""
        return self.graph.has_asymmetric_edge()

    @property
    def is_mutual(self):
        """
        Whether the trajectory exhibits exact-order-two mutual help.

        @ai-generated
        """
        return self.is_interdependent_exactly(2)

    def is_chained(self, length: int = 2):
        """Whether the trajectory exhibits a cooperation chain of at least `length` help edges.

        A chain is a directed temporal trail of help edges whose timestamps never decrease.
        Vertices (agents) may be revisited freely; temporal edges — uniquely identified by
        (helper, beneficiary, t) — may each be used at most once.  Because the help graph at any
        single time step is a finite simple directed graph, and edges at different time steps are
        always distinct triples, every trail is finite.

        A chain encodes two ideas:
           1) transitivity of the cooperation: if a helps b and b helps c, then a also helps c indirectly.
           2) depth of cooperation: i.e. how many subsequent (or simultaneous) cooperative events occur.

        A chain must have at least 2 edges; a single help edge is cooperative but not chained.

        # Returns
        Returns `False` when the longest trail has fewer edges than `length`.

        # Examples
           - `a -> b` returns `False` for the default length;
           - `a -> b -> c` returns `True` for the default length;
           - `a -> b -> c -> a` has a longest trail of length `3`;
           - `a -> b -> c -> d -> b` has a longest trail of length `4`;
           - `a -> b`, and `a -> c` returns `False` (no path of length ≥ 2 from either branch);
           - `a -> b -> a` returns `True`;
           - `a -> b` and `b -> a` at the same time step returns `True` (trail uses each edge once);
           - an independent graph returns `False`.
        """
        if length < 2:
            raise ValueError("A chain must have at least 2 edges")
        return self.graph.longest_trail_length() >= length

    def interdependence_order(self) -> int:
        """
        Return the greatest support size of a temporal closed trail, or `0`.

        @ai-generated
        """
        return self.graph.interdependence_order()

    def is_interdependent_exactly(self, n_agents: int = 2) -> bool:
        """
        Return whether an exact-support closed trail contains exactly `n_agents` agents.

        This exact-order predicate matches `SolveMode.no_interdependence`. It
        is deliberately not monotone across orders.

        @ai-generated
        """
        return self.graph.has_closed_trail_of_order(n_agents)

    def is_interdependent(self, n_agents: int = 2) -> bool:
        """
        Return whether any closed trail has support at least `n_agents`.

        This legacy threshold query is retained for compatibility. Use
        `is_interdependent_exactly` for the non-monotone exact-order contract.

        @ai-generated
        """
        return self.interdependence_order() >= n_agents
