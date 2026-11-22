// sec-runtime-semantics-keyedbindinginitialization: normal true
// Patterns nest in binding and assignment patterns; a nested pattern takes a default when its
// property is undefined, and a default is evaluated only for undefined.
var calls = 0;
function count() { calls++; return { deep: "default" }; }
var { outer: { inner: { deep } } } = { outer: { inner: { deep: "found" } } };
var { gone: { deep: fallback } = count() } = {};
var { present: { deep: kept } = count() } = { present: { deep: "kept" } };
var x, y;
({ p: { q: x = 1, r: y } = { r: 2 } } = {});
deep === "found" && fallback === "default" && kept === "kept" && calls === 1
  && x === 1 && y === 2;
