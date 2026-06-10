# Future optimizations
## No double-encoding of similar trajectories
For one single agent, it is irrelevant to encode both [STAY, SOUTH] and [SOUTH, STAY] since they are equivalent. We could make it such that we do not this twice in the formula.

For multiple agents, we can say that for any position where the agent is the only one that can reach it, we should not encode both [SOUTH, STAY] and [STAY, SOUTH].

To think about: More generally, should we encode one single way to reach every single time-wise position for each agent ? For each combination of all agents ?


## Start tiles
At t=1, it is impossible for any agent to navigate to the start tile of another agent since following conflicts are forbidden.
We could implement that in the time-reachability computation.
