// sec-arrow-function-definitions: normal true
// The parameters of an arrow take defaults and a rest element as those of a function do.
var withDefault = (a, b = a + 1) => a + b;
var restOnly = (...all) => all.length;
var both = (a = 10, ...rest) => a + rest.length;
var named = (f = () => 0) => f.name;
withDefault(1) === 3 && withDefault(1, 5) === 6 && restOnly(1, 2, 3) === 3
  && both() === 10 && both(undefined, 1, 2) === 12 && named() === "f"
  && withDefault.length === 1 && restOnly.length === 0;
