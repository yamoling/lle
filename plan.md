I would like to check properties of the environment similar to what currently exists in the  [cooperation](file:///home/yann/projects/rust/lle/python/lle/cooperation/) module but with the difference that I want to verify wheter **ANY** solution to a given world within a specified time interval has some properties or not.

To do so, I would like to rely on additional constraints or assumptions. I would like to start with a proof of concept on this level:
```text
 .  . S0 S1 .  . 
L0E . .  .  @  .
 .  . .  .  .  .
 .  . .  .  .  .
 X  X .  .  .  .
```

Essentially, any path of length <= 9 requires that agent 0 helps agent 1 to cross the horizontal laser. However, starting from lengths >= 10, the map becomes independent because agent 1 can go behind the wall and avoid the laser beam altogether. Essentially, I would like to implement a series of clauses that encode that. Then, I would like to write a function like `characterize(world: World, t_max: int) -> CooperationProperties` that would return some cooperation properties.

Cooperation properties would describe what properties hold for each path length up to `t_max`. For instance:
- depends(agent0, agent1, 9): i.e. agent 0 depends on agent 1 up to t=9

From such properties, we can deduce that the world becomes independant from t > 9 onward.


Your job is to create a plan for
- the algorithm that must be implemented
- the additional constraints that must be designed to extract the properties out of the map
