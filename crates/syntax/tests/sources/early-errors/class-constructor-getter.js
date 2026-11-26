// sec-class-definitions-static-semantics-early-errors: SyntaxError "a class constructor may not be a getter"
class A { get constructor() {} }
