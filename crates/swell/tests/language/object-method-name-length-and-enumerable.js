// sec-definemethodproperty: normal "m2s1truetruetruetrue"
var o = { m(a, b) {}, "s"() {}, 1() {} };
var d = Object.getOwnPropertyDescriptor(o, "m");
"" + o.m.name + o.m.length + o.s.name + o[1].name + d.writable + d.enumerable + d.configurable + (d.value === o.m);
