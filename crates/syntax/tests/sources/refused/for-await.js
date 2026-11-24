// sec-for-in-and-for-of-statements: refused "for await is not supported before slice 10"
async function f() {
  for await (const x of y);
}
