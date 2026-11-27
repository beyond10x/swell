// sec-object-initializer: SyntaxError "unexpected private name #m"
class C { x = { #m() {} }; }
