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


def test_version():
    import lle
    from lle import __version__

    assert lle.__version__ == __version__
