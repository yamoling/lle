from typing import Literal

SolveModeLiteral = Literal[
    "standard",
    "no-cooperation",
    "no-mutual-cooperation",
    "no-chained-cooperation",
]
# "no-interdependence-N" (N >= 2) is also accepted as a raw str by solve() and ClauseGenerator
