// sec-declarative-environment-records-setmutablebinding-n-v-s: an assignment to a const binding throws, a ReferenceError before its declaration and a TypeError after it
function f() {
  c = 0;
  const c = 1;
  c = 2;
}
