from enum import StrEnum


class LaserMode(StrEnum):
    """Laser behavior modes for the solver."""

    STANDARD = "standard"
    STRICT = "strict"

    def get(self, var, ctx):
        from ._constraints import LaserConstraints, StrictLaserConstraints

        match self:
            case LaserMode.STANDARD:
                return LaserConstraints(var, ctx)
            case LaserMode.STRICT:
                return StrictLaserConstraints(var, ctx)
        raise RuntimeError("Incomplete match-case")
