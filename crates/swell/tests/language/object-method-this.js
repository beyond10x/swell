// sec-method-definitions-runtime-semantics-propertydefinitionevaluation: normal 4
var o = { v: 2, m() { return this.v; } };
o.m() + o["m"]();
