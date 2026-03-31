import lle


def test_from_file():
    lle.from_file("resources/levels/lvl1")


def test_from_str():
    env = lle.from_str("X S0 G .").build()
    assert env.width == 4
    assert env.height == 1


def test_level():
    for lvl in range(1, 7):
        lle.level(lvl).build()
