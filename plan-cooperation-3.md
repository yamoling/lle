# Cooperation characterization

## Current situation
Today, a world can be shown to require cooperation or not by checking whether there exist path that requires no cooperation (i.e. the agents are independent) with a pseudo-code such as the following:

```python
# pseudo-code below
def is_cooperative(world: World, t_min: int, t_max: int):
    path = solve(world, t_min=t_min, t_max=t_max)
    if path is None:
        return "not solvable"
    independent_path = solve_no_cooperation(t_min, t_max)
    if independent_path is not None:
        return "independent"
    return "cooperative"
```

## The vision
We would like to characterize cooperation in a much more fine-grained way. The idea is to build a time-wise agent dependency graph that represents how agents depend on each other to solve a layout.

Essentially, if an agent `a` depends on agent `b` at time step `t`, then there is a directed edge from `(a, t)` to `(b, t)`. From there, if there is a path from a to b at step t1 and from b to c at step t2 >= t1, then we can also say that agent c depends on agent a. The interest of such representation is also that the graph can be "reduced" on its time dimension to provide a snapshot of what cooperation looks like overall (e.g.: "Does `a` ever help `b`?" rather than "Does `a` help `b` at time step `t` ?").

## Definition of dependency
In the scope of LLE, an agent `a` helps agent `b` at time step `t` if blocks a laser of colour `a` and `b` stands on a laser tile without dying (i.e. the beam is blocked for `b`) at time step `t`.

## Properties on the dependency graph
The objective is then to extract properties from the graph in order to characterize cooperation for the level. Some possible properties are the following:
- fan-in: by how many agents the agent is helped. Either across any time step or for a specific given time step `t`, which would mean "Agent `a` is simultaneously helped at time step `t` by `b` and `c`".
- fan-out: how many agents the agent are helped by the current vertex (i.e. agent). Time-wise, or time-agnostic.
- chain depth: `a -> b -> c -> ... -> z`. The length of a chain provides information about the complexity of the cooperation system.
- cycles: `a -> b` at `t`, `b -> c` at `t+1`, then `c -> a` at `t+2` means that all agents depend on each other. Time is important here.
- strongly connected components: when flattened across time, the presence of a strongly connected component means that, to some extent, all agents rely on each other.
- hamiltonian cycles: a cycle across all agents.


### Incomparability of properties
Note that all of the above properties are not ordered, since some properties describe graph structures that are not comparable to others. For instance, the length of a chain can not be compared to the size of a cycle. The fan-out describes something very different from a SCC: a large fan-out means that a specific agent helps many different agents, while a large cycle means that there is a lot of agent-wise interdependency.

As such, these properties should not be ordered.

## Your job
Your job is to implement a cooperation analyser that takes as input the trajectory (a sequence of actions) of a given world, computes the temporal helper graph, and outputs a set of properties on that graph.
