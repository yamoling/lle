Implement this method [@graph.py (131:139)](file:///home/yann/projects/rust/lle/python/lle/characterization/trajectory/graph.py#L131:139) and write tests for it.


# Difference with time agnostic chains
The distinction between "temoral chain" and "time agnostic chain" is important. Let us note help(a, b, t) the predicate that states that agent a helps agent b at time step t. By convention, help(a, b, t{n}) occurs before help(a, b, t{n+m}) with m >= 0. Essentially, t1 occurs after (or at the same time as) t0, and t2 occurs after (or at the same time as) t1.

For instance:
- help(a, b, t0), help(b, c, t1) is a chain of length 1.
- help(a, b, t0), help(b, c, t1), help(c, d, t2) is a chain of length 2.
- help(a, b, t0), help(b, c, t2), help(c, d, t1) is a chain of length 1, even though it may look like a chain of length 2 ! Since help(c, d, t1) occurs before help(b, c, t2), there is no dependency between d and a nor b.


# Implementation approach
One way to implement this functionality is to first build a dependency graph (that takes time into account), and then to find the longest chain. It is up to you to decide if such approach is relevant or if an easier solution exists.
