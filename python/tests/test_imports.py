def test_import_class_from_tiles():
    from lle.tiles import Gem
    from lle.tiles import Laser
    from lle.tiles import LaserSource
    from lle.exceptions import InvalidActionError, InvalidLevelError, InvalidWorldStateError, ParsingError


def test_import_submodule():
    from lle import tiles
    from lle import exceptions


def test_version():
    from lle import __version__

    import lle

    assert lle.__version__ == __version__
