"""World-level interdependence characterization and solver regressions."""

from __future__ import annotations

import pytest
from lle import solver
from lle.characterization import WorldCharacterizer
from lle.characterization.plan import profile_plan

from ...pending import call_or_xfail_unimplemented
from ...world_layouts import (
    BLOCKED_UNSOLVABLE,
    SEQUENCE_3_WITHOUT_CYCLE,
    SEQUENCE_4_WITH_MUTUAL,
    LEVEL_1,
    TWO_AGENT_MUTUAL_REVERSED,
    TWO_AGENT_MUTUAL_WITH_DETOURS,
    InterdependentCase,
    Layout,
    interdependent_cases,
)

INTERDEPENDENT_CASES = interdependent_cases()


def _is_interdependent(wc: WorldCharacterizer, order: int) -> bool:
    """Call the endpoint while classifying its implementation placeholder."""
    return call_or_xfail_unimplemented(wc.is_interdependent, order)


@pytest.mark.parametrize("property_case", INTERDEPENDENT_CASES, ids=lambda c: c.id)
def test_is_interdependent(property_case: InterdependentCase):
    """Check every catalogued positive and negative cycle-order expectation.

    @ai-generated
    """
    wc = WorldCharacterizer(property_case.layout.world(), property_case.t_max)

    assert _is_interdependent(wc, property_case.order) is property_case.expected


def test_interdependence_2_detected_in_canonical_rotation():
    """The canonical temporal-cycle rotation is forbidden by no-interdependence-2."""
    world = TWO_AGENT_MUTUAL_WITH_DETOURS.world()
    plan = solver.solve(world, 6)

    assert plan is not None
    profile = profile_plan(world, plan)
    assert profile.is_interdependent(2)
    earliest = min(profile.graph.edges, key=lambda edge: edge.t)
    assert earliest.helper == 0
    non_interdependent_plan = call_or_xfail_unimplemented(solver.solve, world, 6, mode="no-interdependence-2")
    assert non_interdependent_plan is None


def test_interdependence_2_detected_in_reversed_rotation():
    """The reversed temporal-cycle rotation must also be forbidden.

    @ai-generated
    """
    layout = TWO_AGENT_MUTUAL_REVERSED
    world = layout.world()
    plan = solver.solve(world, 6)

    assert plan is not None
    profile = profile_plan(world, plan)
    assert profile.is_interdependent(2)
    earliest = min(profile.graph.edges, key=lambda edge: edge.t)
    assert earliest.helper == 1
    non_interdependent_plan = call_or_xfail_unimplemented(solver.solve, world, 6, mode="no-interdependence-2")
    assert non_interdependent_plan is None


def test_is_interdependent_rejects_order_below_2():
    """Interdependence orders below two are invalid.

    @ai-generated
    """
    wc = WorldCharacterizer(LEVEL_1.world(), t_max=10)

    for order in [-1, 0, 1]:
        with pytest.raises(ValueError):
            wc.is_interdependent(order)


def test_compute_shortest_non_interdependent_path_rejects_order_below_2():
    """The non-interdependent path query rejects orders below two."""
    wc = WorldCharacterizer(LEVEL_1.world(), t_max=10)

    for order in [-1, 0, 1]:
        with pytest.raises(ValueError):
            wc.compute_shortest_non_interdependent_path(order)


def test_unsolvable_world_is_not_interdependent():
    """Interdependence is undefined for an unsolvable world."""
    world = BLOCKED_UNSOLVABLE.world()
    assert not WorldCharacterizer(world, t_max=10).is_interdependent(2)


def test_is_interdependent_is_cached():
    """A repeated query for one cycle order returns the cached result."""
    wc = WorldCharacterizer(TWO_AGENT_MUTUAL_WITH_DETOURS.world(), t_max=6)

    first = _is_interdependent(wc, 2)
    second = _is_interdependent(wc, 2)

    assert first is second
    assert first


@pytest.mark.parametrize(
    ("layout", "order", "path_expected"),
    [
        pytest.param(SEQUENCE_4_WITH_MUTUAL, 2, False, id="sequence-4-with-mutual-order-2"),
        pytest.param(SEQUENCE_4_WITH_MUTUAL, 3, True, id="sequence-4-with-mutual-order-3"),
        pytest.param(SEQUENCE_3_WITHOUT_CYCLE, 2, True, id="sequence-3-without-cycle-order-2"),
    ],
)
def test_compute_shortest_non_interdependent_path(layout: Layout, order: int, path_expected: bool):
    """Check whether a plan avoiding the requested cycle order exists.

    @ai-generated
    """
    wc = WorldCharacterizer(layout.world(), t_max=layout.expectations[0].t_max)

    path = call_or_xfail_unimplemented(wc.compute_shortest_non_interdependent_path, order)

    assert (path is not None) is path_expected
