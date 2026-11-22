// sec-array.prototype-%symbol.unscopables%: normal "outer-keys,outer-at,function,function"
// sloppy-only
// Inside with ([]) a name Array.prototype[@@unscopables] lists is not a binding of the array (sec-object-environment-records-hasbinding-n), even where the prototype has it; push and indexOf are not listed and are.
Array.prototype.keys = 1;
var keys = "outer-keys", at = "outer-at", push = "outer-push", indexOf = "outer-indexOf";
with ([]) { [keys, at, typeof push, typeof indexOf].join(); }
