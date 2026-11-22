// sec-error.iserror: normal "true,true,true,false,false,false,false"
// Error.isError is true exactly for an object with [[ErrorData]] (sec-error.iserror).
function F() {}
F.prototype = Error.prototype;
var thrown;
try { null.x; } catch (e) { thrown = e; }
[Error.isError(new TypeError("m")), Error.isError(URIError("m")), Error.isError(thrown),
  Error.isError(new F()), Error.isError(Error.prototype), Error.isError({}), Error.isError("Error")].join();
