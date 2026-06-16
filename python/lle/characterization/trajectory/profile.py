from .graph import TemporalDependencyGraph


class TrajectoryProfile:
    def __init__(self, graph: TemporalDependencyGraph) -> None:
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
        Whether the trajectory exhibits mutual help, i.e. help(a, b) and help(b, a).
        """
        edges = self.graph.flattened_edges()
        return any((b, a) in edges for a, b in edges)

    def is_chained(self, length: int = 2):
        """
        Whether the trajectory exhibits a cooperation chain of length `length`.

        # Details
        A chain `(a, t0) -> (b, t1) -> (c, t2) -> ...` is a temporal directed walk whose
        edges progress strictly through time. The length of a chain is the length of the walk,
        i.e. the number of edges that compose it.

        A chain encodes two ideas:
           1) transitivity of the cooperation: if a helps b and b helps c, then a also helps c indirectly.
           2) depth of cooperation: i.e. how many subsequent (or simultaneous) cooperative events occur.

        A chain must have a length of at least 2 edges, otherwise it is not a chain.
        """
        if length < 2:
            raise ValueError("A chain must have at least 2 edges")
        # A chain is a temporal walk whose helpers are distinct and whose timestamps never
        # decrease; its final beneficiary is unconstrained, so a walk may close back onto one
        # or multiple earlier agents (a cycle `a -> b -> c -> a -> c` or a lasso `a -> b -> c -> d -> b`).
        return self.graph.longest_walk() == length

    def interdependence_order(self) -> int:
        """The order of the largest temporal cycle in this trajectory (0 if none)."""
        return self.graph.max_temporal_cycle_order()

    def is_interdependent(self, n_agents: int = 2) -> bool:
        """
        Whether this trajectory's dependency graph contains a cycle of length >= `n_agents`.
        """
        return self.graph.max_temporal_cycle_order() >= n_agents
