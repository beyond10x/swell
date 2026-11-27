// sec-scripts-static-semantics-early-errors: SyntaxError "new.target outside a function"
class A { [new.target]() {} }
