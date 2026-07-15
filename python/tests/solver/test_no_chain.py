import pytest
from lle import solve
from lle.solver import SolveMode

from ..world_layouts import ChainedCase, chained_cases


@pytest.mark.parametrize("test_case", chained_cases(), ids=lambda case: case.id)
def test_no_chain_mode_matches_world_specification(test_case: ChainedCase):
    """The mode is unsatisfiable exactly when the chain length is unavoidable."""
    plan = solve(
        test_case.layout.world(),
        test_case.t_max,
        mode=SolveMode.no_chain(test_case.length),
    )
    assert (plan is None) is test_case.expected
