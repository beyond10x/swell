// sec-string.prototype.trim: normal "[a b]|true|true|5|0"
// With sec-trimstring: WhiteSpace and LineTerminator removed at both ends (trim), at the start
// (sec-string.prototype.trimstart) or at the end (sec-string.prototype.trimend), kept inside.
var s = "\t  ﻿a b　\n ";
"[" + s.trim() + "]|" + (s.trimStart() === "a b　\n ") + "|" +
  (s.trimEnd() === "\t  ﻿a b") + "|" + "᠎a b᠎".trim().length + "|" +
  " \r\v\f".trim().length;
