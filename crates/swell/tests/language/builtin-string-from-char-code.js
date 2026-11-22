// sec-string.fromcharcode: normal "AB,true,65,true"
// ToUint16 (sec-touint16) takes each argument modulo 2^16; a code point above 0xFFFF is
// two code units through String.fromCodePoint (sec-string.fromcodepoint).
var r = [String.fromCharCode(65, 0x10042)];
r.push(String.fromCharCode(-1) === "\uFFFF");
r.push(String.fromCharCode(65.9).charCodeAt(0));
r.push(String.fromCodePoint(0x1F600) === String.fromCharCode(0xD83D, 0xDE00));
r.join();
