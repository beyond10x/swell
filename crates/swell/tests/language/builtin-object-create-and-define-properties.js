// sec-object.create: normal "true,1,2,1,false,true,false,xy,1,2,true,false"
// Object.create with properties and Object.defineProperties (sec-objectdefineproperties): every descriptor is read before one is defined, so one that is not valid defines nothing.
var proto = { p: 1 };
var o = Object.create(proto, { a: { value: 1, enumerable: true }, b: { get: function () { return 2; } } });
var n = Object.create(null);
var log = "";
var props = { get x() { log += "x"; return { value: 1 }; }, get y() { log += "y"; return { value: 2 }; } };
var t = Object.defineProperties({}, props);
var target = {};
var bad = false;
try { Object.defineProperties(target, { a: { value: 1 }, b: 1 }); } catch (e) { bad = e instanceof TypeError; }
[Object.getPrototypeOf(o) === proto, o.a, o.b, o.p, Object.getOwnPropertyDescriptor(o, "a").writable,
 Object.getPrototypeOf(n) === null, "toString" in n, log, t.x, t.y, bad, target.hasOwnProperty("a")].join();
