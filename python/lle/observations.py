"""Observation types and observation generators for `LLE`.

This module defines the public observation presets accepted by the Python API
and the internal generators that turn a `World` into arrays or images.
Use `ObservationType.from_str(...)` when you accept user input, and use
`get_observation_generator(...)` to build the concrete generator for a world.
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum
from typing import Literal, Sequence

import numpy as np
import numpy.typing as npt

from lle.world import World, WorldState

from .types import AgentId, Position

ObservationTypeLiteral = Literal[
    "layered",
    "flattened",
    "partial3x3",
    "partial5x5",
    "partial7x7",
    "state",
    "rgb-image",
    "perspective",
    "normalized-state",
    "layered-padded-1",
    "layered-padded-2",
    "layered-padded-3",
    "layered-padded",
]


class ObservationType(str, Enum):
    """Public observation presets supported by the environment."""

    NORMALIZED_STATE = "normalized-state"
    STATE = "state"
    """The state of the world (agents' positions, alive status, gems collections) as a flat vector."""
    RGB_IMAGE = "rgb-image"
    """The rendered world as an RGB image."""
    LAYERED = "layered"
    """
    Layered observations of the map (walls, lasers, ...) as shown below. Only 2 agents are shown for the sake of clarity.

    ![Layered representation of the world](../../docs/layers.png)
    """
    FLATTENED = "flattened"
    """The layered representation flattened to one dimension."""
    PARTIAL_3x3 = "partial3x3"
    PARTIAL_5x5 = "partial5x5"
    PARTIAL_7x7 = "partial7x7"
    LAYERED_PADDED = "layered-padded"
    LAYERED_PADDED_1AGENT = "layered-padded-1"
    LAYERED_PADDED_2AGENTS = "layered-padded-2"
    LAYERED_PADDED_3AGENTS = "layered-padded-3"
    PERSPECTIVE = "perspective"

    @staticmethod
    def from_str(s: ObservationTypeLiteral | str) -> "ObservationType":
        return ObservationType(s)

    def get_observation_generator(self, world: World, padding_size: int = 0, n_groups: int | None = None) -> "ObservationGenerator":
        """Build the generator for this preset.

        `padding_size` reserves channels for agents that the world does not have,
        and `n_groups` does the same for lift/button groups. Both exist to pin the
        observation shape when a single network has to consume several maps: leave
        `n_groups` at `None` to size the lift/button section after the world.
        """
        match self:
            case ObservationType.NORMALIZED_STATE:
                return StateGenerator(world, normalize=True)
            case ObservationType.STATE:
                return StateGenerator(world, normalize=False)
            case ObservationType.RGB_IMAGE:
                return RGBImage(world)
            case ObservationType.LAYERED:
                return Layered(world, n_groups)
            case ObservationType.FLATTENED:
                return FlattenedLayered(world, n_groups)
            case ObservationType.PARTIAL_3x3:
                return PartialGenerator(world, 3, n_groups)
            case ObservationType.PARTIAL_5x5:
                return PartialGenerator(world, 5, n_groups)
            case ObservationType.PARTIAL_7x7:
                return PartialGenerator(world, 7, n_groups)
            case ObservationType.LAYERED_PADDED:
                return LayeredPadded(world, padding_size, n_groups)
            case ObservationType.LAYERED_PADDED_1AGENT:
                return LayeredPadded(world, 1, n_groups)
            case ObservationType.LAYERED_PADDED_2AGENTS:
                return LayeredPadded(world, 2, n_groups)
            case ObservationType.LAYERED_PADDED_3AGENTS:
                return LayeredPadded(world, 3, n_groups)
            case ObservationType.PERSPECTIVE:
                return PerspectiveLayered(world, n_groups)
            case other:
                raise ValueError(f"Unknown observation type: {other}")


def _build_group_index(world: World) -> tuple[list[int], dict[int, int]]:
    """Map the (possibly sparse) lift/button group ids onto dense channel indices.

    Lifts and buttons share the index so that `LIFT_0 + g` and `BUTTON_0 + g`
    designate the same group, which is exactly the button-pulses-lift relation.
    """
    group_ids = sorted({tile.group_id for tile in (*world.lifts, *world.buttons)})
    return group_ids, {group_id: index for index, group_id in enumerate(group_ids)}


def _resolve_n_groups(group_ids: list[int], n_groups: int | None) -> int:
    """The number of lift/button group channels to reserve.

    `None` sizes the section after the world it is given. An explicit value pins
    it instead, so that maps with different group counts still yield the same
    observation shape — the counterpart of `padding_size` for agents.
    """
    if n_groups is None:
        return len(group_ids)
    if n_groups < len(group_ids):
        raise ValueError(f"n_groups={n_groups} is too small for a world with {len(group_ids)} lift/button groups")
    return n_groups


def _assert_world_fits(
    new_world: World,
    n_new_groups: int,
    n_agents: int,
    n_groups: int,
    world_dims: tuple[int, ...] | None,
):
    """Reject a `set_world` target that would change the observation shape."""
    if new_world.n_agents > n_agents:
        raise ValueError(
            f"The new world has {new_world.n_agents} agents but only {n_agents} channels are reserved. "
            "Rebuild the generator, or reserve more room with `padding_size`."
        )
    if n_new_groups > n_groups:
        raise ValueError(
            f"The new world has {n_new_groups} lift/button groups but only {n_groups} are reserved. "
            "Rebuild the generator, or reserve more room with `n_groups`."
        )
    new_dims = (new_world.height, new_world.width, new_world.layers)
    if world_dims is not None and tuple(world_dims) != new_dims:
        raise ValueError(f"The new world is {new_dims} but the generator is built for {tuple(world_dims)}.")


def _authorized_agents(authorized_agent_id: AgentId | None, n_agents: int) -> list[AgentId]:
    """The agents allowed to use a lift or button. `None` means all of them."""
    if authorized_agent_id is None:
        return list(range(n_agents))
    if authorized_agent_id < n_agents:
        return [authorized_agent_id]
    return []


def require_unshared_colours(world: World, generator_name: str) -> None:
    """Refuse to build a non-centred observation for a world where two agents share a colour.

    Agents of one colour are indistinguishable in such an observation: they appear in the same
    layer, and nothing in the array says which of them the observation is addressed to. Only
    `perspective`, which centres the map on the observing agent, stays unambiguous.
    """
    seen = dict[int, int]()
    for agent_num, colour in enumerate(world.agent_colours):
        if colour in seen:
            raise ValueError(
                f"{generator_name} cannot represent a world where agents share a colour: agents "
                f"{seen[colour]} and {agent_num} both have colour {colour}, so neither could tell "
                f'which of them the observation describes. Use the "perspective" observation, '
                "which centres the map on the observing agent."
            )
        seen[colour] = agent_num


@dataclass
class ObservationGenerator(ABC):
    """Base class for world-to-observation converters."""

    def __init__(self, world: World):
        super().__init__()
        self._world = world

    @abstractmethod
    def observe(self) -> npt.NDArray[np.float32]:
        """Return the observation for every agent."""

    def get_state(self) -> npt.NDArray[np.float32]:
        return self.observe()[0]

    def to_world_state(self, data: npt.NDArray[np.float32]) -> WorldState:
        """Convert observation data back into a `WorldState`.

        Generators that cannot reconstruct a world state should override this method.
        """
        raise NotImplementedError(f"This method is not implemented for {self.__class__.__name__}")

    @property
    @abstractmethod
    def obs_type(self) -> ObservationType:
        """The observation preset represented by this generator."""

    @property
    @abstractmethod
    def shape(self) -> tuple[int, ...]:
        """The shape of a single-agent observation."""

    def set_world(self, new_world: World):
        """Point the generator at another world."""
        self._world = new_world

    def reset(self) -> None:
        """Refresh any state cached across calls to `observe()` (e.g. static layers derived
        from world topology). Generators that cache such data must be told when it may have
        changed: `LLE.reset()` calls this after every episode reset (covering e.g.
        `randomize_lasers`), and any code that mutates world topology directly (e.g.
        `world.exit_pos = ...`) outside of `LLE` must call it explicitly afterwards. The
        default implementation is a no-op; generators with no such cache need not override it.
        """


class StateGenerator(ObservationGenerator):
    def __init__(self, world: World, normalize: bool):
        super().__init__(world)
        self.n_gems = world.n_gems
        self.n_agents = world.n_agents
        if normalize:
            self.dimensions = np.array([world.height, world.width, world.layers] * world.n_agents)
        else:
            self.dimensions = np.ones(world.n_agents * len(world.world_dims))

    def observe(self):
        state = self._world.get_state().as_array()
        state[: self._world.n_agents * WorldState.POSITION_SIZE] = (
            state[: self._world.n_agents * WorldState.POSITION_SIZE] / self.dimensions
        )
        return np.tile(state, reps=(self._world.n_agents, 1))

    def to_world_state(self, data):
        data[: self._world.n_agents * WorldState.POSITION_SIZE] = data[: self._world.n_agents * WorldState.POSITION_SIZE] * self.dimensions
        return WorldState.from_array(data.tolist(), self.n_agents, self.n_gems)

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.STATE

    @property
    def shape(self):
        """The full world state: (i, j, k) for each agent, each gem's collection status, and each agent's alive flag."""
        return (self._world.n_agents * WorldState.AGENT_SIZE + self.n_gems,)

    @property
    def unit_size(self) -> int:
        return 2


class RGBImage(ObservationGenerator):
    def __init__(self, world: World):
        super().__init__(world)
        self._shape = tuple(world.get_image().shape)

    def observe(self):
        obs = self._world.get_image()
        return np.tile(obs, (self._world.n_agents, 1, 1, 1)).astype(np.float32)

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.RGB_IMAGE

    @property
    def shape(self):
        return self._shape


@dataclass
class LayeredPadded(ObservationGenerator):
    """Layered observations with an optional agent and lift/button group budget."""

    def __init__(self, world: World, padding_size: int, n_groups: int | None = None, *, guard_shared_colours: bool = True):
        super().__init__(world)
        if guard_shared_colours:
            require_unshared_colours(world, type(self).__name__)
        self.width = world.width
        self.height = world.height
        self.n_agents = world.n_agents + padding_size
        self.group_ids, self.group_index = _build_group_index(world)
        self.n_groups = _resolve_n_groups(self.group_ids, n_groups)
        # One agent layer and one laser layer per colour. Sizing the bands by the colour space
        # rather than by `n_agents` is what keeps a laser colour above `n_agents` from aliasing
        # into the WALL band.
        self.n_colours = world.n_colours + padding_size
        self.A0 = 0
        self.LASER_0 = self.A0 + self.n_colours
        self.WALL = self.LASER_0 + self.n_colours
        self.VOID = self.WALL + 1
        self.GEM = self.VOID + 1
        self.EXIT = self.GEM + 1
        # One channel per lift group (sign = direction), one per button group, and
        # one per agent telling whether that agent may use the lift/button below it.
        # The whole section is absent when no group is reserved.
        self.LIFT_0 = self.EXIT + 1
        self.BUTTON_0 = self.LIFT_0 + self.n_groups
        self.AUTH_0 = self.BUTTON_0 + self.n_groups
        n_channels = self.AUTH_0 + (self.n_agents if self.n_groups > 0 else 0)
        self._shape = (n_channels, world.height, world.width, world.layers)
        self.ordered_gem_pos = sorted(gem.pos for gem in world.gems)

        self.static_obs = self._setup()

    def set_world(self, new_world: World):
        """Point the generator at another world and rebuild its static layers.

        Raises `ValueError` when the new world would not produce the same
        observation shape: a network whose input width silently changes
        mid-training is a failure that has to be loud.
        """
        group_ids, group_index = _build_group_index(new_world)
        _assert_world_fits(new_world, len(group_ids), self.n_agents, self.n_groups, self._shape[1:])
        super().set_world(new_world)
        self.group_ids, self.group_index = group_ids, group_index
        self.width = new_world.width
        self.height = new_world.height
        self.ordered_gem_pos = sorted(gem.pos for gem in new_world.gems)
        self.static_obs = self._setup()

    def _setup(self):
        """Compute the layers that are constant for an episode: walls and voids never change;
        exit positions and laser source positions/colours only change (if at all) via
        `world.reset()`-adjacent events (e.g. `randomize_lasers`) or explicit topology edits
        (e.g. `world.exit_pos = ...`), both of which call `reset()` on this generator — see
        `ObservationGenerator.reset`. Recomputed there instead of on every `observe()` call.

        Lift/button positions, groups and authorizations never change either, so they are
        baked in here too.
        """
        obs = np.zeros(self._shape, dtype=np.float32)
        for i, j, k in self._world.wall_pos:
            obs[self.WALL, i, j, k] = 1.0

        for i, j, k in self._world.void_pos:
            obs[self.VOID, i, j, k] = 1.0

        # Neither the position of a lift/button nor its group or its authorized
        # agent changes during an episode, so all of it can be baked into the
        # static layers just like walls/voids.
        for lift in self._world.lifts:
            i, j, k = lift.pos
            obs[self.LIFT_0 + self.group_index[lift.group_id], i, j, k] = 1.0 if lift.direction == "U" else -1.0
            self._encode_authorization(obs, lift.authorized_agent_id, lift.pos)

        for button in self._world.buttons:
            i, j, k = button.pos
            obs[self.BUTTON_0 + self.group_index[button.group_id], i, j, k] = 1.0
            self._encode_authorization(obs, button.authorized_agent_id, button.pos)

        for i, j, k in self._world.exit_pos:
            obs[self.EXIT, i, j, k] = 1.0

        for source in self._world.laser_sources:
            i, j, k = source.pos
            obs[self.LASER_0 + source.colour, i, j, k] = -1.0

        return obs

    def _encode_authorization(self, obs: npt.NDArray[np.float32], authorized_agent_id: AgentId | None, pos: Position):
        """Flag which agents may use the lift or button standing at `pos`."""
        i, j, k = pos
        for agent_id in _authorized_agents(authorized_agent_id, self.n_agents):
            obs[self.AUTH_0 + agent_id, i, j, k] = 1.0

    def reset(self) -> None:
        self.static_obs = self._setup()

    def to_world_state(self, data: npt.NDArray[np.float32]) -> WorldState:
        """Reconstruct a world state from a layered observation.

        This assumes that all agents are alive.
        """
        _, i, j, k = np.nonzero(data[self.A0 : self.A0 + self.n_colours])
        agents_positions = [(int(i[n]), int(j[n]), int(k[n])) for n in range(self._world.n_agents)]
        gems_collected = []
        # We need the gem positions to be ordered because they are initially stored in a hashmap
        for i, j, k in self.ordered_gem_pos:
            gems_collected.append(bool(data[self.GEM, i, j, k] == 0.0))
        return WorldState(agents_positions, gems_collected)

    def observe(self):
        obs = np.copy(self.static_obs)
        for laser in self._world.lasers:
            i, j, k = laser.pos
            if laser.is_on:
                obs[self.LASER_0 + laser.colour, i, j, k] = 1.0
        for gem in self._world.gems:
            i, j, k = gem.pos
            if not gem.is_collected:
                obs[self.GEM, i, j, k] = 1.0
        # Every agent is stamped into its colour's layer. Two agents never share a cell, so
        # same-colour agents cannot overwrite each other.
        for colour, (y, x, z) in zip(self._world.agent_colours, self._world.agents_positions):
            obs[self.A0 + colour, y, x, z] = 1.0
        return np.tile(obs, (self._world.n_agents, 1, 1, 1, 1))

    @property
    def shape(self):
        return self._shape

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.LAYERED


class Layered(LayeredPadded):
    def __init__(self, world: World, n_groups: int | None = None, *, guard_shared_colours: bool = True):
        super().__init__(world, padding_size=0, n_groups=n_groups, guard_shared_colours=guard_shared_colours)


class FlattenedLayered(ObservationGenerator):
    def __init__(self, world: World, n_groups: int | None = None):
        super().__init__(world)
        self.layered = Layered(world, n_groups)
        size = 1
        for s in self.layered.shape:
            size = size * s
        self._shape = (size,)

    def observe(self):
        obs = self.layered.observe()
        return obs.reshape(self._world.n_agents, -1)

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.FLATTENED

    @property
    def shape(self):
        return self._shape

    @property
    def unit_size(self) -> int:
        return 0

    def set_world(self, new_world: World):
        self.layered.set_world(new_world)
        return super().set_world(new_world)


class PartialGenerator(ObservationGenerator):
    def __init__(self, world: World, square_size: int, n_groups: int | None = None):
        super().__init__(world)
        assert square_size % 2 == 1, "Can only use odd numbers for the square size"
        require_unshared_colours(world, type(self).__name__)
        self.size = square_size
        self._center = self.size // 2
        self.n_agents = world.n_agents
        # One layer per colour for the agents, one per colour for the lasers, walls, gems, exits
        self.n_colours = world.n_colours
        self.group_ids, self.group_index = _build_group_index(world)
        self.n_groups = _resolve_n_groups(self.group_ids, n_groups)
        # Each agent, walls, each laser, gems, exits, then the lift/button section:
        # one channel per lift group (sign = direction), one per button group and
        # one per agent for the authorizations. Absent when no group is reserved.
        self.WALL = self.n_colours
        self.LASER_0 = self.WALL + 1
        self.GEM = self.LASER_0 + self.n_colours
        self.EXIT = self.GEM + 1
        self.LIFT_0 = self.EXIT + 1
        self.BUTTON_0 = self.LIFT_0 + self.n_groups
        self.AUTH_0 = self.BUTTON_0 + self.n_groups
        n_channels = self.AUTH_0 + (world.n_agents if self.n_groups > 0 else 0)
        self._shape = (n_channels, self.size, self.size)

    def set_world(self, new_world: World):
        """Point the generator at another world, refusing one that would reshape it.

        The window size is fixed, so unlike `LayeredPadded` the map dimensions are
        free to change; only the agent and group budgets have to hold.
        """
        group_ids, group_index = _build_group_index(new_world)
        _assert_world_fits(new_world, len(group_ids), self.n_agents, self.n_groups, None)
        super().set_world(new_world)
        self.group_ids, self.group_index = group_ids, group_index

    @property
    def shape(self) -> tuple[int, int, int]:
        return self._shape

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.PARTIAL_3x3

    def encode_layer(self, layer: npt.NDArray[np.float32], origin: Position, positions: Sequence[Position], fill_value: float = 1.0):
        if len(positions) == 0:
            return
        for i, j, k in positions:
            # The window is a 2D slice centred on the agent: tiles sitting on
            # another floor must not be projected onto it.
            if k != origin[2]:
                continue
            i, j = i - origin[0] + self._center, j - origin[1] + self._center
            if 0 <= i < self.size and 0 <= j < self.size:
                layer[i, j] = fill_value

    def _encode_authorization(
        self,
        obs: npt.NDArray[np.float32],
        origin: Position,
        authorized_agent_id: AgentId | None,
        pos: Position,
    ):
        """Flag which agents may use the lift or button standing at `pos`."""
        for agent_id in _authorized_agents(authorized_agent_id, self.n_agents):
            self.encode_layer(obs[self.AUTH_0 + agent_id], origin, [pos])

    def observe(self) -> npt.NDArray[np.float32]:
        obs = np.zeros((self._world.n_agents, *self._shape), dtype=np.float32)
        colours = self._world.agent_colours
        for a, agent_pos in enumerate(self._world.agents_positions):
            # Agents positions, grouped by colour
            for colour, other_pos in zip(colours, self._world.agents_positions):
                self.encode_layer(obs[a, colour], agent_pos, [other_pos])
            # Gems
            self.encode_layer(obs[a, self.GEM], agent_pos, [gem.pos for gem in self._world.gems if not gem.is_collected])
            # Exits
            self.encode_layer(obs[a, self.EXIT], agent_pos, [exit_pos for exit_pos in self._world.exit_pos])
            # Walls
            self.encode_layer(obs[a, self.WALL], agent_pos, [wall_pos for wall_pos in self._world.wall_pos])
            # Lasers
            laser_positions = self._get_lasers_positions()
            for colour, positions in laser_positions.items():
                self.encode_layer(obs[a, self.LASER_0 + colour], agent_pos, positions)
            # Laser sources
            for source in self._world.laser_sources:
                self.encode_layer(obs[a, self.LASER_0 + source.colour], agent_pos, [source.pos], fill_value=-1.0)
            # Lifts: one channel per group, direction encoded as the sign
            for lift in self._world.lifts:
                self.encode_layer(
                    obs[a, self.LIFT_0 + self.group_index[lift.group_id]],
                    agent_pos,
                    [lift.pos],
                    fill_value=1.0 if lift.direction == "U" else -1.0,
                )
                self._encode_authorization(obs[a], agent_pos, lift.authorized_agent_id, lift.pos)
            # Buttons: one channel per group
            for button in self._world.buttons:
                self.encode_layer(obs[a, self.BUTTON_0 + self.group_index[button.group_id]], agent_pos, [button.pos])
                self._encode_authorization(obs[a], agent_pos, button.authorized_agent_id, button.pos)
        return obs

    def _get_lasers_positions(self) -> dict[AgentId, list[Position]]:
        """Beam tiles that are currently on, grouped by colour."""
        laser_positions = dict[AgentId, list[Position]]()
        for laser in self._world.lasers:
            if laser.is_on:
                lasers = laser_positions.get(laser.colour, [])
                lasers.append(laser.pos)
                laser_positions[laser.colour] = lasers
        return laser_positions


class PerspectiveLayered(ObservationGenerator):
    """Full-observability layered observation, centred on the observing agent.

    Unlike `Layered`, this stays unambiguous when several agents share a colour, because the
    observing agent is identified by its position rather than by a layer index:

    - the canvas is `(2·height − 1, 2·width − 1)` and the observing agent sits at its exact
      centre, so the whole map fits wherever the agent stands;
    - cells outside the map are marked in the `WALL` layer, so an agent near an edge sees the
      outside as impassable rather than as empty space;
    - there is one agent layer and one laser layer per colour, and every agent is stamped into
      its colour's layer, exactly as lasers already were;
    - the observing agent's colour is transposed with colour 0, so an agent always sees itself
      as colour 0 regardless of the colour it was assigned;
    - it also carries the lift/button group section, like `LayeredPadded`. The `AUTH_0` band is
      per-agent-identity rather than per-colour, so it is spatially shifted like everything else
      but never colour-permuted.
    """

    def __init__(self, world: World, n_groups: int | None = None):
        super().__init__(world)
        self.height = world.height
        self.width = world.width
        self.n_colours = world.n_colours
        self.n_agents = world.n_agents
        self.group_ids, self.group_index = _build_group_index(world)
        self.n_groups = _resolve_n_groups(self.group_ids, n_groups)
        self.A0 = 0
        self.LASER_0 = self.A0 + self.n_colours
        self.WALL = self.LASER_0 + self.n_colours
        self.VOID = self.WALL + 1
        self.GEM = self.VOID + 1
        self.EXIT = self.GEM + 1
        self.LIFT_0 = self.EXIT + 1
        self.BUTTON_0 = self.LIFT_0 + self.n_groups
        self.AUTH_0 = self.BUTTON_0 + self.n_groups
        n_channels = self.AUTH_0 + (self.n_agents if self.n_groups > 0 else 0)
        self._shape = (n_channels, 2 * world.height - 1, 2 * world.width - 1, world.layers)
        # The map-sized view this one shifts and permutes. It is built unguarded on purpose:
        # sharing a colour is exactly the case this generator exists to handle.
        self._layered = Layered(world, n_groups, guard_shared_colours=False)

    @property
    def shape(self):
        return self._shape

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.PERSPECTIVE

    def set_world(self, new_world: World):
        self._layered.set_world(new_world)
        return super().set_world(new_world)

    def reset(self) -> None:
        self._layered.reset()

    def observe(self):
        # The map-sized layers, then one shifted, colour-canonicalised copy per agent.
        world_view = self._layered.observe()[0]
        obs = np.zeros((self._world.n_agents, *self._shape), dtype=np.float32)
        # Everything outside the map is a wall; the map window is cleared per agent below.
        obs[:, self.WALL] = 1.0
        colours = self._world.agent_colours
        for agent_num, (i, j, _) in enumerate(self._world.agents_positions):
            di, dj = self.height - 1 - i, self.width - 1 - j
            window = self._permute_colours(world_view, colours[agent_num])
            obs[agent_num, :, di : di + self.height, dj : dj + self.width, :] = window
        return obs

    def _permute_colours(self, world_view: npt.NDArray[np.float32], colour: int):
        """Swap `colour` into slot 0 of both the agent band and the laser band."""
        if colour == 0:
            return world_view
        permuted = np.copy(world_view)
        for band in (self.A0, self.LASER_0):
            permuted[[band, band + colour]] = permuted[[band + colour, band]]
        return permuted
