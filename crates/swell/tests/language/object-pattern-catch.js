// sec-runtime-semantics-catchclauseevaluation: normal true
// A catch parameter pattern binds its names in the scope of the catch block only, with
// defaults, and a closure in the block sees them.
var message = "outer";
var read;
try {
  throw { message: "inner", code: 7 };
} catch ({ message, code, extra = code + 1 }) {
  read = function () { return message + code + extra; };
}
message === "outer" && read() === "inner78";
