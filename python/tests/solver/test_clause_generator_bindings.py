import pytest
from lle import Action, World
from lle.solver.clauses import ClauseGenerator, SolveMode
from pysat.solvers import Minisat22

COOPERATION_DELTA_WORLD = "L0E .  .  X\nL1E .  .  X\nL2E .  .  X\nS0  S1 S2 X"


def _formula_is_sat(clauses: list[list[int]], assumptions: list[int]) -> bool:
    with Minisat22(bootstrap_with=clauses) as solver:
        return bool(solver.solve(assumptions=assumptions))


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
def test_delta_stream_matches_fresh_complete_formulas_across_modes(
    world_string: str,
    mode: SolveMode,
    horizons: tuple[int, ...],
    collect_gems: bool,
):
    """One retained solver, advanced through ascending horizons, agrees with a fresh complete
    formula at every horizon.

    @ai-generated
    """
    t_max = max(horizons)
    stream_generator = ClauseGenerator(World(world_string), t_max)
    stream_generator.start_delta_stream(mode=mode, collect_gems=collect_gems)
    with Minisat22() as incremental_solver:
        for horizon in horizons:
            delta_clauses, delta_assumptions = stream_generator.advance_delta_stream(horizon)
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


def test_delta_stream_supports_incremental_solving_and_repeated_horizons():
    """Returned deltas can be accumulated while current assumptions select the objective.

    @ai-generated
    """
    generator = ClauseGenerator(World("S0 . . X"), 4)
    generator.start_delta_stream()
    with Minisat22() as solver:
        clauses, assumptions = generator.advance_delta_stream(2)
        solver.append_formula(clauses)
        assert not solver.solve(assumptions=assumptions)  # pyright: ignore[reportUnknownMemberType]

        repeated, repeated_assumptions = generator.advance_delta_stream(2)
        assert repeated == []
        assert repeated_assumptions == assumptions

        clauses, assumptions = generator.advance_delta_stream(3)
        solver.append_formula(clauses)
        assert solver.solve(assumptions=assumptions)  # pyright: ignore[reportUnknownMemberType]
        model = solver.get_model()  # pyright: ignore[reportUnknownMemberType]
        assert model is not None
        plan = generator.decode_plan(model, 3)
        assert plan == [[Action.EAST], [Action.EAST], [Action.EAST]]


def test_advance_delta_stream_without_start_raises():
    """Advancing before starting a stream is a clear user error, not a silent no-op."""
    generator = ClauseGenerator(World("S0 . . X"), 4)
    with pytest.raises(ValueError, match="no delta stream is active"):
        generator.advance_delta_stream(2)


def test_advance_delta_stream_rejects_decreasing_horizon():
    """A stream's horizon may only grow; going backwards should start a new stream instead."""
    generator = ClauseGenerator(World("S0 . . X"), 4)
    generator.start_delta_stream()
    generator.advance_delta_stream(3)
    with pytest.raises(ValueError, match="cannot go backwards"):
        generator.advance_delta_stream(2)


def test_starting_a_new_stream_resets_the_cursor():
    """Starting a new stream (even with the same mode) redelivers the full prefix.

    This is what makes it safe to call `start_delta_stream` once per retained SAT solver
    instance: a solver that has never seen any clause always gets the complete prefix,
    regardless of what a previous, now-abandoned stream on the same generator already sent.

    @ai-generated
    """
    generator = ClauseGenerator(World("S0 . . X"), 4)
    generator.start_delta_stream()
    first_clauses, _ = generator.advance_delta_stream(3)
    assert first_clauses

    generator.start_delta_stream()
    second_clauses, _ = generator.advance_delta_stream(3)
    # Same clause count as the first stream: everything is resent, not skipped as "already
    # delivered". The objective's activation literal differs between streams (it is minted
    # fresh each time), so the two clause lists are equivalent rather than byte-identical.
    assert len(second_clauses) == len(first_clauses)


def test_new_stream_on_a_reused_generator_serves_a_fresh_solver_correctly():
    """One generator backing two independent streams (and two independent solvers) in sequence
    must serve each solver its own complete prefix.

    This mirrors `WorldCharacterizer`, which runs `find_shortest` once per cooperation mode on
    one shared `Solver`/generator, each call retaining its own SAT solver. It is the regression
    test for the bug the delta-stream API was redesigned to rule out: a generator that thought a
    previous stream's deliveries also covered a new one would silently starve the new stream's
    solver of clauses it has never actually seen.

    @ai-generated
    """
    world_string = COOPERATION_DELTA_WORLD
    t_max = 4
    generator = ClauseGenerator(World(world_string), t_max)

    for mode in (SolveMode.standard(), SolveMode.no_sequence(2)):
        generator.start_delta_stream(mode=mode)
        with Minisat22() as incremental_solver:
            for horizon in (2, 3, 4):
                clauses, assumptions = generator.advance_delta_stream(horizon)
                incremental_solver.append_formula(clauses)
                incremental_is_sat = bool(
                    incremental_solver.solve(assumptions=assumptions)  # pyright: ignore[reportUnknownMemberType]
                )

                fresh = ClauseGenerator(World(world_string), t_max)
                expected = fresh.generate(horizon, mode=mode)
                assert incremental_is_sat is _formula_is_sat(*expected), (
                    f"delta/full mismatch for {mode} at horizon {horizon} on a reused generator"
                )


def test_delta_stream_repeated_request_returns_no_clause():
    """Repeating a request returns nothing new and the assumptions minted the first time.

    @ai-generated
    """
    generator = ClauseGenerator(World(COOPERATION_DELTA_WORLD), 3)
    generator.start_delta_stream(mode=SolveMode.no_sequence(2))
    _, first = generator.advance_delta_stream(3)
    repeated, repeated_assumptions = generator.advance_delta_stream(3)
    assert repeated == []
    assert repeated_assumptions == first


def test_delta_stream_normalizes_structurally_impossible_modes():
    """A structurally impossible restriction opens a standard stream: no extra assumption."""
    generator = ClauseGenerator(World("S0 . X"), 2)
    generator.start_delta_stream(mode="no-cooperation")
    _, assumptions = generator.advance_delta_stream(2)
    assert len(assumptions) == 1


def test_full_generation_does_not_affect_delta_streams():
    """Complete-formula calls and the incremental stream remain independent.

    @ai-generated
    """
    generator = ClauseGenerator(World("S0 . . X"), 4)
    generator.start_delta_stream()
    first_clauses, _ = generator.advance_delta_stream(2)
    assert first_clauses

    generator.generate(2)

    repeated, _ = generator.advance_delta_stream(2)
    assert repeated == []
