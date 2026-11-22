// sec-string.prototype.tolowercase: normal "hello, world 42!|HELLO, WORLD 42!|abc|ABC|SyntaxError"
// Then sec-string.prototype.touppercase, sec-string.prototype.tolocalelowercase and
// sec-string.prototype.tolocaleuppercase: the 26 letters mapped, every other ASCII unit kept; a
// code unit of 0x80 or above is refused before slice 5 (docs/design/engine.md, section 12).
var s = "Hello, World 42!";
var refusal;
try { "é".toUpperCase(); } catch (e) { refusal = e.name; }
s.toLowerCase() + "|" + s.toUpperCase() + "|" + "AbC".toLocaleLowerCase() + "|" +
  "aBc".toLocaleUpperCase() + "|" + refusal;
