import pytest
from lle import Action, World
from lle.solver.clauses import ClauseGenerator, SolveMode
from pysat.solvers import Minisat22  # pyright: ignore[reportMissingTypeStubs]

COOPERATION_DELTA_WORLD = "L0E .  .  X\nL1E .  .  X\nL2E .  .  X\nS0  S1 S2 X"


def _formula_is_sat(clauses: list[list[int]], assumptions: list[int]) -> bool:
    """Solve one generated formula without retaining state between calls.

    @ai-generated
    """
    with Minisat22(bootstrap_with=clauses) as solver:
        return bool(solver.solve(assumptions=assumptions))  # pyright: ignore[reportUnknownMemberType]


def test_generate_default_preserves_full_prefix_api():
    """The new flag defaults to the legacy complete-prefix result.

    @ai-generated
    """
    generator = ClauseGenerator(World("S0 . . X"), 4)
    implicit = generator.generate(3)
    explicit = generator.generate(3, only_delta=False)
    assert implicit == explicit


@pytest.mark.parametrize(
    ("world_string", "mode", "horizons", "collect_gems"),
    [
        pytest.param("S0 G . X", SolveMode.standard(), (1, 2, 3), True, id="standard-with-gems"),
        pytest.param(
            COOPERATION_DELTA_WORLD,
            SolveMode.no_cooperation(),
            (2, 3, 4),
            False,
            id="no-cooperation",
        ),
        pytest.param(
            COOPERATION_DELTA_WORLD,
            SolveMode.no_asymmetric(),
            (2, 3, 4),
            False,
            id="no-asymmetric",
        ),
        pytest.param(
            COOPERATION_DELTA_WORLD,
            SolveMode.no_sequence(2),
            (2, 3, 4),
            False,
            id="no-sequence",
        ),
        pytest.param(
            COOPERATION_DELTA_WORLD,
            SolveMode.no_interdependence(2),
            (2, 3, 4),
            False,
            id="no-interdependence",
        ),
        pytest.param(
            COOPERATION_DELTA_WORLD,
            SolveMode.no_convergence(2),
            (2, 3, 4),
            False,
            id="no-convergence",
        ),
        pytest.param(
            COOPERATION_DELTA_WORLD,
            SolveMode.no_divergence(2),
            (2, 3, 4),
            False,
            id="no-divergence",
        ),
        pytest.param(
            COOPERATION_DELTA_WORLD,
            SolveMode.no_fully_coupled(),
            (2, 3, 4),
            False,
            id="no-fully-coupled",
        ),
    ],
)
def test_only_delta_matches_fresh_complete_formulas_across_modes(
    world_string: str,
    mode: SolveMode,
    horizons: tuple[int, ...],
    collect_gems: bool,
):
    """Accumulated permanent deltas match fresh complete formulas at every horizon.

    @ai-generated
    """
    t_max = max(horizons)
    delta_generator = ClauseGenerator(World(world_string), t_max)
    with Minisat22() as incremental_solver:
        for horizon in horizons:
            delta_clauses, delta_assumptions = delta_generator.generate(
                horizon,
                mode=mode,
                collect_gems=collect_gems,
                only_delta=True,
            )
            incremental_solver.append_formula(delta_clauses)
            delta_is_sat = bool(
                incremental_solver.solve(assumptions=delta_assumptions)  # pyright: ignore[reportUnknownMemberType]
            )

            fresh_generator = ClauseGenerator(World(world_string), t_max)
            full_clauses, full_assumptions = fresh_generator.generate(
                horizon,
                mode=mode,
                collect_gems=collect_gems,
            )
            assert delta_is_sat is _formula_is_sat(full_clauses, full_assumptions), f"delta/full mismatch for {mode} at horizon {horizon}"


def test_only_delta_supports_incremental_solving_and_repeated_horizons():
    """Returned deltas can be accumulated while current assumptions select the objective.

    @ai-generated
    """
    generator = ClauseGenerator(World("S0 . . X"), 4)
    with Minisat22() as solver:
        clauses, assumptions = generator.generate(2, only_delta=True)
        solver.append_formula(clauses)
        assert not solver.solve(assumptions=assumptions)  # pyright: ignore[reportUnknownMemberType]

        repeated, repeated_assumptions = generator.generate(2, only_delta=True)
        assert repeated == []
        assert repeated_assumptions == assumptions

        clauses, assumptions = generator.generate(3, only_delta=True)
        solver.append_formula(clauses)
        assert solver.solve(assumptions=assumptions)  # pyright: ignore[reportUnknownMemberType]
        model = solver.get_model()  # pyright: ignore[reportUnknownMemberType]
        assert model is not None
        plan = generator.decode_plan(model, 3)
        assert plan == [[Action.EAST], [Action.EAST], [Action.EAST]]


@pytest.mark.parametrize(
    ("t", "mode", "collect_gems"),
    [
        (1, "standard", False),
        (3, "no-cooperation", False),
        (3, "standard", True),
    ],
)
def test_only_delta_rejects_incompatible_stream_transitions(t: int, mode: str, collect_gems: bool):
    """Decreasing horizons, effective-mode changes, and gem-policy changes are explicit errors.

    @ai-generated
    """
    generator = ClauseGenerator(World("L0E .  G X\nS0  S1 . X"), 4)
    generator.generate(2, only_delta=True)
    with pytest.raises(ValueError, match="Incompatible only_delta generation request"):
        generator.generate(t, mode=mode, collect_gems=collect_gems, only_delta=True)


def test_only_delta_compares_modes_after_layout_normalization():
    """Structurally impossible restrictions share the normalized standard stream.

    @ai-generated
    """
    generator = ClauseGenerator(World("S0 . X"), 2)
    generator.generate(2, mode="no-cooperation", only_delta=True)
    clauses, assumptions = generator.generate(2, mode="standard", only_delta=True)
    assert clauses == []
    assert len(assumptions) == 1


def test_only_delta_rejects_parameterized_mode_changes():
    """A parameterized mode's value is part of the incremental stream identity.

    @ai-generated
    """
    world = World("L0E .  .  X\nL1E .  .  X\nS0  S1 S2 X")
    generator = ClauseGenerator(world, 4)
    generator.generate(2, mode=SolveMode.no_sequence(2), only_delta=True)
    with pytest.raises(ValueError, match="Incompatible only_delta generation request"):
        generator.generate(3, mode=SolveMode.no_sequence(3), only_delta=True)


def test_full_generation_does_not_advance_delta_stream():
    """Legacy calls and the stateful delta cursor remain independent.

    @ai-generated
    """
    generator = ClauseGenerator(World("S0 . . X"), 4)
    generator.generate(2)
    clauses, assumptions = generator.generate(2, only_delta=True)
    assert clauses
    assert assumptions
