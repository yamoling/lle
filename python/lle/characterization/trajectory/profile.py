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
    def is_mutual(self):
        """
        Whether the trajectory exhibits mutual help, i.e. help(a, b) and help(b, a).
        """
        return len(self.graph.strongly_connected_components()) > 0

    @property
    def is_chained(self):
        """
        Whether the trajectory exhibits a chained help pattern, i.e. help(a, b) -> help(b, c) -> ...
        """
        return self.graph.longest_chain() >= 2 or len(self.graph.strongly_connected_components()) > 0

    def interdependence_order(self) -> int:
        """The order of the largest temporal cycle in this trajectory (0 if none)."""
        return self.graph.max_temporal_cycle_order()

    def is_interdependent(self) -> bool:
        """Whether this trajectory's dependency graph contains a temporal cycle."""
        return self.graph.max_temporal_cycle_order() >= 2
