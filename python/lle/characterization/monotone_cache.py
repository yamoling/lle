from __future__ import annotations

from dataclasses import dataclass, field
from typing import TypeVar

T = TypeVar("T")


@dataclass
class MonotoneCache:
    """
    Cache such that once cache[t]=True, we know that cache[t']=True for all t' <= t, and
    once cache[t]=False, we know that cache[t']=False for all t' >= t.

    For instance:
      - if we know that there is a chain of length 5, there is also a chain of length 4, 3, 2 and 1.
      - if there is no chain of length 5, we know that there is no chain of length >= 5
    """

    values: dict[int, bool] = field(default_factory=dict)

    def __setitem__(self, order: int, value: bool):
        self.values[order] = value

    def get(self, order: int):
        """Infer a cached solver result for ``order`` from monotone neighbours."""
        for cached_order, value in self.values.items():
            if value and cached_order >= order:
                return value
            if not value and cached_order <= order:
                return value
        return None
