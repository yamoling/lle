# Constrained procedural world generation
The objective of this feature is to enable the user to generate a series of randomly generated worlds that satisfy some property.

## Details
The user should be able to express some constraints intuitively such as: "The generated worlds must be require cooperation within a t_max steps".

Your job is:
- to design a user-friendly way to express the properties that the generated worlds should have
- to implement the generation loop
- to write tests that check that the generation function works as intended for multiple combinations of properties. 

## World characterization
A world characterization procedure already exists in the `lle.characterization` module. The `WorldCharacterizer` class plays this role.

There is probably room for improvement in the "user-friendlyness" of the implementation. 
Notably, it is currently not possible to express the properties of a World as an object; it is only possible to compute them.

## Glimpse at the final result
The user should be able to write something like the following:
```python
import lle

# n = 1 by default
world = lle.generate(mutual=True)

# generator when n > 1, uses n_cpus - 1 parallel jobs
for world in lle.generate(n=100, cooperative=True):
    print(world)

# One single world, but spawns 4 parallel jobs
world = lle.generate(n_jobs=4, n_lasers=3, n_agents=4)

world_properties = <something you have to figure out>
worlds = list(lle.generate(properties, n=10)) # A list of 10 worlds that match the given properties
```


## Advices
- You should use the world characterizer for the procedural generation.
- Use function overloading to make the `generate` function intuitive and user-friendly
- the current implementation of "generate" is a good starting point, but key elements of the `generator` base function may have to be updated to use the `characterize_world` function.
