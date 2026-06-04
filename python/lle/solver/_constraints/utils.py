from typing import Any, Generator

from pysat.formula import Atom, Equals, Formula, Implies, XOr


def _int2atom(v: int | Formula) -> Formula:
    if isinstance(v, Formula):
        return v
    return Atom(abs(v)) if v > 0 else ~Atom(abs(v))  # type: ignore


def equals(a: int | Formula, b: int | Formula) -> Generator[list[int], Any, None]:
    if not isinstance(a, Formula):
        a = _int2atom(a)
    if not isinstance(b, Formula):
        b = _int2atom(b)
    f = Equals(a, b)
    f.clausify()
    yield from f.clauses


def implies(a: int | Formula, b: int | Formula) -> Generator[list[int], Any, None]:
    if not isinstance(a, Formula):
        a = _int2atom(a)
    if not isinstance(b, Formula):
        b = _int2atom(b)
    f = Implies(a, b)
    f.clausify()
    yield from f.clauses


def xor(a: int | Formula, b: int | Formula, *vars: int | Formula) -> Generator[list[int], Any, None]:
    if not isinstance(a, Formula):
        a = _int2atom(a)
    if not isinstance(b, Formula):
        b = _int2atom(b)
    f = XOr(a, b, *(_int2atom(v) for v in vars))
    f.clausify()
    yield from f.clauses
