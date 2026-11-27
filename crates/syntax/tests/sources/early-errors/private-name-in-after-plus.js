// sec-relational-operators: SyntaxError "unexpected private name #x"
class C { #x; m(o) { return 1 + #x in o; } }
