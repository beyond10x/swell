// sec-string.prototype.codepointat: normal "b,98,128512,56832,,true"
// charAt and charCodeAt (sec-string.prototype.charat, sec-string.prototype.charcodeat) read one
// code unit; codePointAt reads a surrogate pair, or the trailing half on its own.
var s = "ab\uD83D\uDE00";
var r = [s.charAt(1), s.charCodeAt(1), s.codePointAt(2), s.codePointAt(3)];
r.push(s.charAt(4), s.codePointAt(4) === undefined);
r.join();
