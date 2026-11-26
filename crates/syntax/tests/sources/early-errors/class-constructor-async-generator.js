// sec-class-definitions-static-semantics-early-errors: SyntaxError "a class constructor may not be an async generator method"
class A { async *constructor() {} }
