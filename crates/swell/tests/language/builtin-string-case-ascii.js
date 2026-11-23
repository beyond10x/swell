// sec-string.prototype.tolowercase: normal "hello, world 42!|HELLO, WORLD 42!|abc|ABC|É"
// Then sec-string.prototype.touppercase, sec-string.prototype.tolocalelowercase and
// sec-string.prototype.tolocaleuppercase: the 26 letters mapped, every other ASCII unit kept.
var s = "Hello, World 42!";
s.toLowerCase() + "|" + s.toUpperCase() + "|" + "AbC".toLocaleLowerCase() + "|" +
  "aBc".toLocaleUpperCase() + "|" + "é".toUpperCase();
