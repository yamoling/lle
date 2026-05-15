from pysat.formula import CNF


class SATModel:
    def __init__(self):
        self.cnf = CNF()

    def add_clause(self, clause):
        self.cnf.append(clause)

    def extend(self, clauses):
        self.cnf.clauses.extend(clauses)
