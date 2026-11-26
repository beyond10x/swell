// sec-delete-operator-static-semantics-early-errors: SyntaxError "delete of an unqualified identifier in strict mode code"
class A { get m() { return delete x; } }
