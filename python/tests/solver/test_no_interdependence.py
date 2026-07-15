import pytest
from lle import solve

from ..pending import call_or_xfail_unimplemented
from ..world_layouts import InterdependentCase, interdependent_cases


@pytest.mark.parametrize("test_case", interdependent_cases(), ids=lambda case: case.id)
def test_no_interdependence_matches_specification(test_case: InterdependentCase):
    """The mode is unsatisfiable exactly when the cycle order is unavoidable."""
    plan = call_or_xfail_unimplemented(
        solve,
        test_case.layout.world(),
        test_case.t_max,
        mode=f"no-interdependence-{test_case.order}",
    )
    assert (plan is None) is test_case.expected
