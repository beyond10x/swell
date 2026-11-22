// sec-variable-statement-runtime-semantics-evaluation: normal true
// A var pattern binds each name to the property it names, by shorthand, identifier, string or
// number key, and a name declared twice takes the last value.
var o = { a: 1, b: 2, "c d": 3, 4: 5 };
var { a, b: renamed, "c d": spaced, 4: four } = o, { a: twice, b: twice } = o;
var { missing } = o;
var {} = o;
function inner() {
  var { a } = { a: 10 };
  return a;
}
a === 1 && renamed === 2 && spaced === 3 && four === 5 && twice === 2
  && missing === undefined && inner() === 10 && a === 1;
