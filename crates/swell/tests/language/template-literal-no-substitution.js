// sec-template-literals-runtime-semantics-evaluation: normal true
// A NoSubstitutionTemplate is its TV, a String; a `$` not before `{` and a `{` are characters.
var empty = ``;
var plain = `abc`;
var dollar = `$a{b}$`;
var quotes = `'"`;
empty === "" && plain === "abc" && dollar === "$a{b}$" && quotes === "'\"" && typeof `x` === "string";
