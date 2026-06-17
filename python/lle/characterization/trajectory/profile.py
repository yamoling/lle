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
        """Whether the trajectory exhibits a cooperation chain of at least `length` help edges.

        A chain is a temporal directed walk of help edges whose timestamps never decrease and never repeat. The
        length is the number of help edges. Agents and lasers may repeat freely, so simultaneous
        mutual help, cycles, and lassos all count. If same-timestep help contains a directed cycle,
        the chain length is unbounded.

        A chain encodes two ideas:
           1) transitivity of the cooperation: if a helps b and b helps c, then a also helps c indirectly.
           2) depth of cooperation: i.e. how many subsequent (or simultaneous) cooperative events occur.

        A chain must have a length of at least 2 edges; a single help edge is cooperative but not chained.
        """
        if length < 2:
            raise ValueError("A chain must have at least 2 edges")
        return self.graph.longest_walk() >= length

    def interdependence_order(self) -> int:
        """The order of the largest temporal cycle in this trajectory (0 if none)."""
        return self.graph.max_temporal_cycle_order()

    def is_interdependent(self, n_agents: int = 2) -> bool:
        """
        Whether this trajectory's dependency graph contains a cycle of length >= `n_agents`.
        """
        return self.graph.max_temporal_cycle_order() >= n_agents
