// sec-object.defineproperty: normal "1falsefalsefalse"
// A field absent from the descriptor of a new property is false (sec-validateandapplypropertydescriptor, step 2).
var o = {};
Object.defineProperty(o, "x", { value: 1 });
var d = Object.getOwnPropertyDescriptor(o, "x");
"" + d.value + d.writable + d.enumerable + d.configurable;
