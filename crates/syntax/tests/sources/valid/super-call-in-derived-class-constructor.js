// sec-class-definitions-static-semantics-early-errors: parses
class A extends B { constructor() { super(); (() => super())(); } }
