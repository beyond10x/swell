// sec-performeval: normal true
var o = {};
eval(o) === o && eval(1) === 1 && eval(true) === true && eval(null) === null &&
  (0, eval)(o) === o;
