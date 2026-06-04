def implies(a: int, b: int):
    """
    a -> b (¬a ∨ b)
    """
    yield [-a, b]


def equals(a: int, b: int):
    """
    a <-> b means: (a -> b) ^ (b -> a)
    """
    yield from implies(a, b)
    yield from implies(b, a)


def xor(a: int, b: int):
    """
    a ⊕ b means (a ∨ b) ∧ (¬a ∨ ¬b)
    """
    yield [a, b]
    yield [-a, -b]
