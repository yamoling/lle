"""Verify that all ```python fenced code blocks in package docstrings don't raise."""

from __future__ import annotations

import ast
import re
import textwrap
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent.parent
LLE_PKG = ROOT / "python" / "lle"

_FENCE = re.compile(r"```python\n(.*?)```", re.DOTALL)

# Any block touching these symbols invokes the SAT solver and is slow.
_SLOW_KEYWORDS = ("lle.generate(", "lle.solve(", "lle.is_cooperative(", "lle.characterize(")


def _collect_from(path: Path) -> list[tuple[str, str]]:
    """Return (id, code) pairs for every python fenced block in path's docstrings."""
    source = path.read_text()
    try:
        tree = ast.parse(source, filename=str(path))
    except SyntaxError:
        return []

    nodes: list[tuple[int, str, str]] = []
    for node in ast.walk(tree):
        if not isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            continue
        docstring = ast.get_docstring(node)
        if not docstring:
            continue
        name = getattr(node, "name", "<module>")
        lineno = getattr(node, "lineno", 0)
        nodes.append((lineno, name, docstring))

    nodes.sort()
    rel = path.relative_to(ROOT)
    results: list[tuple[str, str]] = []
    for _, name, docstring in nodes:
        for i, code in enumerate(_FENCE.findall(docstring)):
            block_id = f"{rel}::{name}[{i}]"
            results.append((block_id, textwrap.dedent(code)))
    return results


def _collect_from_md(path: Path) -> list[tuple[str, str]]:
    """Return (id, code) pairs for every python fenced block in a Markdown file."""
    source = path.read_text()
    rel = path.relative_to(ROOT)
    return [(f"{rel}[{i}]", textwrap.dedent(code)) for i, code in enumerate(_FENCE.findall(source))]


def _all_blocks() -> list[tuple[str, str]]:
    blocks: list[tuple[str, str]] = []
    for py_file in sorted(LLE_PKG.rglob("*.py")) + sorted(LLE_PKG.rglob("*.pyi")):
        blocks.extend(_collect_from(py_file))
    blocks.extend(_collect_from_md(ROOT / "readme.md"))
    return blocks


_ALL = _all_blocks()


def _param(block_id: str, code: str):
    marks = [pytest.mark.slow] if any(kw in code for kw in _SLOW_KEYWORDS) else []
    return pytest.param(code, id=block_id, marks=marks)


# Imports prelude, such that code examples can use the documented imports
_PRELUDE = """
from lle import World, Action, EventType, WorldState, WorldEvent
import lle
"""
GLOBALS = {}
exec(compile(_PRELUDE, "<prelude>", "exec"), GLOBALS)


@pytest.mark.parametrize("code", [_param(*b) for b in _ALL])
def test_docblock(code: str) -> None:

    exec(compile(code, "<docblock>", "exec"), GLOBALS)
