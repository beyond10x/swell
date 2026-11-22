// sec-template-literals-runtime-semantics-evaluation: normal true
// A template inside a substitution, an object literal and a function whose braces close inside
// it, and a template that spans lines.
var nested = `a${`b${1 + 1}c`}d`;
var deep = `${`${`${"x"}`}`}`;
var braces = `${{ k: "v" }.k}${function () { return "}"; }()}`;
var lines = `1${
  "2"
}3`;
nested === "ab2cd" && deep === "x" && braces === "v}" && lines === "123";
