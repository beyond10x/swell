// sec-createunmappedargumentsobject: normal "1/2/3,true,true,false,true"
// An arguments object, unmapped in the strict run and mapped in the sloppy one
// (sec-createmappedargumentsobject), has an own [Symbol.iterator] whose value is
// %Array.prototype.values%, writable, not enumerable and configurable; its iterator steps the
// arguments in order.
function f() {
  var it = arguments[Symbol.iterator](), out = [], s;
  while (!(s = it.next()).done) out.push(s.value);
  var d = Object.getOwnPropertyDescriptor(arguments, Symbol.iterator);
  return [out.join("/"), d.value === Array.prototype.values, d.writable, d.enumerable,
          d.configurable].join();
}
f(1, 2, 3);
