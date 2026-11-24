// sec-async-arrow-function-definitions-runtime-semantics-evaluation: an async arrow with an expression body resolves with its value, and one with a block body takes this from its scope
var a = async (p) => await p;
var b = async () => {
  return this;
};
