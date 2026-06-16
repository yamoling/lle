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
        Whether the trajectory exhibits a chained help pattern of length >= `length`.

        Exemple: help(a, b, t), help(b, c, t+1), ...
        """
        if length < 2:
            raise ValueError("A chain must have at least 2 edges")
        return self.graph.longest_chain() >= length

    def interdependence_order(self) -> int:
        """The order of the largest temporal cycle in this trajectory (0 if none)."""
        return self.graph.max_temporal_cycle_order()

    def is_interdependent(self, n_agents: int = 2) -> bool:
        """
        Whether this trajectory's dependency graph contains a cycle of length >= `n_agents`.
        """
        return self.graph.max_temporal_cycle_order() >= n_agents
