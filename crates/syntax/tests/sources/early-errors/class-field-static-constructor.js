// sec-class-definitions-static-semantics-early-errors: SyntaxError "a static class field may not be named constructor"
class C { static 'constructor' = 1; }
