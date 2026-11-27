// sec-relational-operators: SyntaxError "unexpected private name #y"
class C { #x; #y; m(o) { return #x in #y in o; } }
