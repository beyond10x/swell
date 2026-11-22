// sec-switch-statement-static-semantics-early-errors: parses
switch (0) {
  case 0:
    let a = 1;
    break;
  default:
    const b = a;
}
