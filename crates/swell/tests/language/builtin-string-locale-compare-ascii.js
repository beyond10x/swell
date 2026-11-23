// sec-string.prototype.localecompare: normal "-1,1,0,-1,1|-1"
// Strings of ASCII code units in the order of their code units, and "é" after "a" (0x61 before
// 0xE9, docs/design/engine.md, decision 5).
[
  "a".localeCompare("b"),
  "b".localeCompare("a"),
  "a".localeCompare("a"),
  "A".localeCompare("a"),
  "ab".localeCompare("a")
].join() + "|" + "a".localeCompare("é");
