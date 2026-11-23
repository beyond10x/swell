// sec-regular-expression-literals-runtime-semantics-evaluation: normal true
var r = /a(b)?/g;
Object.prototype.toString.call(r) === "[object RegExp]" && r.source === "a(b)?" &&
  r.flags === "g" && r.lastIndex === 0 && r !== /a(b)?/g;
