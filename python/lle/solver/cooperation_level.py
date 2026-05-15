"""Public CooperationLevel enum used by lle.cooperation_level and the cooperative generators."""

from __future__ import annotations

from enum import Enum


class CooperationLevel(str, Enum):
    """Precise classification of a world's cooperation requirement.

    Members inherit from ``str`` so that ``CooperationLevel.COOPERATIVE == "cooperative"``
    and the values can be passed straight to APIs that expect plain strings.

    Members fall into three groups:

    * ``UNSOLVABLE`` — the world cannot be solved at all within ``t_max``.
    * ``INDEPENDENT`` — solvable without any agent ever shielding another.
    * ``COOPERATIVE`` and below — cooperation is required; the remaining
      members refine the *shape* of the dependency structure.
    """

    UNSOLVABLE = "unsolvable"
    INDEPENDENT = "independent"
    COOPERATIVE = "cooperative"
    ASYMMETRIC = "asymmetric"
    MUTUAL = "mutual"
    CHAIN = "chain"
    DISTRIBUTED = "distributed"
    FULLY_COUPLED = "fully_coupled"

    @classmethod
    def cooperative_subtypes(cls) -> tuple["CooperationLevel", ...]:
        """Return the levels that imply cooperation is required."""
        return (
            cls.COOPERATIVE,
            cls.ASYMMETRIC,
            cls.MUTUAL,
            cls.CHAIN,
            cls.DISTRIBUTED,
            cls.FULLY_COUPLED,
        )


__all__ = ["CooperationLevel"]
