def test_import_class_from_tiles():
    from lle import __version__
    from lle.tiles import Gem, Laser, LaserSource

    assert isinstance(__version__, str)

    from lle import tiles

    assert tiles.Gem == Gem
    assert tiles.Laser == Laser
    assert tiles.LaserSource == LaserSource

    from lle.exceptions import InvalidActionError, InvalidLevelError, InvalidWorldStateError, ParsingError  # noqa: F401


def test_import_submodule():
    from lle import exceptions, tiles, world  # noqa: F401


def test_import_from_submodules():
    from lle.tiles import Direction, Gem, Laser, LaserSource  # noqa: F401
    from lle.types import AgentId, LaserId, Position  # noqa: F401
    from lle.world import Action, EventType, World, WorldEvent, WorldState  # noqa: F401


def test_import_submodule_from_world():
    from lle.world.rendering import TILE_SIZE  # noqa: F401


def test_solver_imports():
    from lle.solver import SolveMode, clauses  # noqa: F401
    from lle.solver.clauses import ClauseGenerator, SolveMode  # noqa: F401, F811


def test_version():
    import lle
    from lle import __version__

    assert lle.__version__ == __version__


def test_characterization_helpers_are_exported_from_both_paths():
    """The package root re-exports the characterization helpers themselves."""
    import lle
    import lle.characterization as characterization

    for name in ("is_cooperative", "is_asymmetric", "is_chained", "is_convergent", "is_divergent"):
        assert getattr(lle, name) is getattr(characterization, name)
        assert name in lle.__all__
        assert name in characterization.__all__
