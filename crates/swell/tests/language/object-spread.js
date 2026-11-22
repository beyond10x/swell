// sec-copydataproperties: normal true
// A spread copies the own enumerable properties of its source into the literal; undefined and
// null copy nothing.
var base = { a: 1, b: 2 };
Object.defineProperty(base, "hidden", { value: 3, enumerable: false });
var copy = { z: 0, ...base, ...undefined, ...null, b: 9 };
Object.keys(copy).join() === "z,a,b" && copy.a === 1 && copy.b === 9 && base.b === 2
  && !("hidden" in copy);
