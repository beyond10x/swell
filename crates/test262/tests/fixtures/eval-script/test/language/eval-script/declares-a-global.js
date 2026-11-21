// Written for the tests of swell-test262.
/*---
description: >
  $262.evalScript runs its source as a script of the realm of the function: its var and function
  declarations become properties of that realm's global object (ScriptEvaluation,
  GlobalDeclarationInstantiation), and the script of another realm's evalScript declares in
  that realm only.
---*/

$262.evalScript("var declared = 1; function declaredFunction() { return 2; }");
assert.sameValue(declared, 1, "the var of the script");
assert.sameValue(declaredFunction(), 2, "the function of the script");
assert.sameValue(this.declared, 1, "a property of the global object");

var other = $262.createRealm();
other.evalScript("var inOther = 3;");
assert.sameValue(typeof inOther, "undefined", "not declared in this realm");
assert.sameValue(other.global.inOther, 3, "declared in the other realm");
