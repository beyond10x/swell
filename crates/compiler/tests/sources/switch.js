// sec-switch-statement: the case tests in source order, then the default; the bodies fall through
switch (a) {
  case 1: b;
  default: c;
  case 2: d;
}
