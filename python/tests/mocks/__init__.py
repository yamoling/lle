from .mock_solver import MockSolver


def fail_if_called(*args, **kwargs):
    raise AssertionError(f"""This function should not be called.
        args: {args}
        kwargs:{kwargs}""")


__all__ = ["MockSolver"]
