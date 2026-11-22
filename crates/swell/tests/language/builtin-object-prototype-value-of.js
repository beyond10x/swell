// sec-object.prototype.valueof: normal "true,object,true,true,[object Object]|vs"
// Object.prototype.valueOf is ToObject of the this value; OrdinaryToPrimitive (sec-ordinarytoprimitive) calls it before toString, and the object it answers is not a primitive.
var o = {};
var v = Object.prototype.valueOf;
var r = [o.valueOf() === o, typeof v.call(1), v.call("s") instanceof String];
try { v.call(undefined); } catch (e) { r.push(e instanceof TypeError); }
var log = "";
var p = { valueOf: function () { log += "v"; return v.call(this); }, toString: function () { log += "s"; return "[object Object]"; } };
r.push(p + "|" + log);
r.join();
