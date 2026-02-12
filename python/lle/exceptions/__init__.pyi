import typing

@typing.final
class InvalidActionError(ValueError):
    r"""
    Raised when the action taken by an agent is invalid or when the number of actions provided is different from the number of agents.
    """

    ...

@typing.final
class InvalidLevelError(ValueError):
    r"""
    Raised when the level asked does not exist.
    """

    ...

@typing.final
class InvalidWorldStateError(ValueError):
    r"""
    Raised when the state of the world is invalid.
    """

    ...

@typing.final
class ParsingError(ValueError):
    r"""
    Raised when there is a problem while parsing a world string.
    """

    ...
