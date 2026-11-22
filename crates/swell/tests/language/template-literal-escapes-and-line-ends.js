// sec-static-semantics-tv: normal true
// The TV of an escape is its SV; a LineContinuation is empty; a CR LF and a CR in the source
// are each a LF, LS and PS are kept (the sources with CR are given to eval, which reads them).
var escapes = `\n\t\x41B\u{43}\u{1F600}\0\`\$\{\\\a`;
var continued = `a\
b`;
var crlf = eval("`a\r\nb`");
var cr = eval("`a\rb`");
var lf = eval("`a\nb`");
var ls = eval("`a b`");
var continuedCrlf = eval("`a\\\r\nb`");
var dollarBrace = `\${x}`;
escapes === "\n\tAB\x43😀\0`${\\a" && continued === "ab" && crlf === "a\nb" && cr === "a\nb" && lf === "a\nb" && ls === "a b" && continuedCrlf === "ab" && dollarBrace === "${x}";
