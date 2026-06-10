from __future__ import annotations

from dataclasses import dataclass

from ..characterization.world_characterization import WorldCharacterizer
from ..world import World


@dataclass(frozen=True)
class WorldFilter:
    """Behavioral constraints for generated worlds.

    All boolean fields accept ``True`` (must have the property), ``False``
    (must not have it), or ``None`` (no constraint).  The filter is picklable
    so it can be sent to worker processes.
    """

    cooperative: bool | None = None
    """
    - ``True``: the world must *require* laser cooperation (no independent plan exists within ``t_max``).
    - ``False``: the world must be independently solvable.
    """

    mutual: bool | None = None
    """
    - ``True``: the world must require mutual cooperation (every agent both helps and is helped).
    - ``False``: mutual cooperation must not be required.
    """

    t_max: int | None = None
    """Override the ``t_max`` used for characterisation.  If ``None``, the generator's own ``t_max`` is used."""

    def is_satisfied_by(self, world: World, default_t_max: int) -> bool:
        """Return whether ``world`` satisfies every constraint in this filter."""
        effective_t_max = self.t_max if self.t_max is not None else default_t_max
        c = WorldCharacterizer(world, effective_t_max)
        if not c.is_solvable:
            return False
        if self.cooperative is not None and c.is_cooperative != self.cooperative:
            return False
        if self.mutual is not None and c.is_mutual != self.mutual:
            return False
        return True

    @property
    def indepdenent(self):
        if self.cooperative is None:
            return None
        return not self.cooperative
