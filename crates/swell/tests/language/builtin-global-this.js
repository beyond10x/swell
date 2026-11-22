// sec-globalthis: normal "true,object,true,true,false,true"
// globalThis is the global this value, writable, configurable and not enumerable (sec-globalthis).
var d = Object.getOwnPropertyDescriptor(this, "globalThis");
var inner = (function () { return globalThis; })();
[globalThis === this, typeof globalThis, d.writable, d.configurable, d.enumerable, inner === this].join();
