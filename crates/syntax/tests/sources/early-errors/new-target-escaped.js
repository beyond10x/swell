// sec-grammar-notation: SyntaxError "keyword target must not contain escaped characters"
function f() {
  new.t\u0061rget;
}
