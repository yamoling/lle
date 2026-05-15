"""Public CooperationLevel enum used by lle.cooperation_level and the cooperative generators."""

from __future__ import annotations

from enum import Enum


class CooperationLevel(str, Enum):
    """Precise classification of a world's cooperation requirement.

    Members inherit from ``str`` so that ``CooperationLevel.COOPERATIVE == "cooperative"``
    and the values can be passed straight to APIs that expect plain strings.

    *Cooperation* here means an agent of colour ``c`` blocks (shields) a laser
    of colour ``c`` so that a teammate can cross it. The dependency edge
    *helper → beneficiary* is recorded whenever the helper sits on its own
    beam while the beneficiary is somewhere downstream on that same beam.
    From the resulting dependency graph we read off the structural shape:

    Solvability gate:

    * ``UNSOLVABLE`` — no joint plan reaches all exits within ``t_max``.
    * ``INDEPENDENT`` — solvable under strict-laser semantics, so no agent ever
      has to block another. Cooperation is *not* required.

    Cooperative subtypes (cooperation required; ordered from most-structured
    to least-structured, matching the classifier's decision order):

    * ``FULLY_COUPLED`` — every agent belongs to a single strongly connected
      component in the dependency graph (everyone helps everyone, at least
      transitively). Requires at least two agents.
    * ``MUTUAL`` — at least one pair of agents help each other (the dependency
      graph contains a bidirectional edge ``a ↔ b``).
    * ``DISTRIBUTED`` — at least one beneficiary depends on two or more
      distinct helpers (indegree ≥ 2 in the dependency graph).
    * ``CHAIN`` — the dependency edges form a single linear chain
      ``a → b → c → …`` of length ≥ 2, with no branching or merging.
    * ``ASYMMETRIC`` — there is at least one helper → beneficiary edge but the
      structure is neither chain-like nor distributed (typically a single
      directed edge, never reciprocated).
    * ``COOPERATIVE`` — cooperation is required by the strict-laser test, yet
      no helper event was observed on the SAT-found plan (the catch-all when
      no other subtype applies).
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
