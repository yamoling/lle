class SATModel:
    def __init__(self):
        from pysat.formula import CNF  # pyright: ignore[reportMissingImports]

        self.cnf = CNF()

    def add_clause(self, clause):
        self.cnf.append(clause)

    def extend(self, clauses):
        self.cnf.clauses.extend(clauses)
