// sec-string.prototype.localecompare: normal "-1,1,0,-1,1|SyntaxError"
// Strings of ASCII code units in the order of their code units; a code unit of 0x80 or above
// is refused before slice 5 (docs/design/engine.md, section 12).
var refusal;
try { "a".localeCompare("é"); } catch (e) { refusal = e.name; }
[
  "a".localeCompare("b"),
  "b".localeCompare("a"),
  "a".localeCompare("a"),
  "A".localeCompare("a"),
  "ab".localeCompare("a")
].join() + "|" + refusal;
