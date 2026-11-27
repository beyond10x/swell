// sec-relational-operators: SyntaxError "unexpected private name #x"
class C { #x; m() { return #x; } }
