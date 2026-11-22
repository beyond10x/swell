// sec-string.raw: normal true
// String.raw as a tag reads the raw strings of the template object with the substitutions
// between them: each escape stays as written, a NotEscapeSequence included.
var a = String.raw`a\n${1}b\u0041${"c"}\xg`;
var b = String.raw`\\${2}`;
a === "a\\n1b\\u0041c\\xg" && b === "\\\\2" && String.raw`` === "";
