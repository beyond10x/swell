// sec-property-accessors: normal "b3"
var o = { f: function () { return this === o; } };
o["f"]() && "abc"[1] + "abc".length;
