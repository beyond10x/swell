// sec-await: an await in an async generator, resumed, and the end of the body that settles the request at the head
async function* g() {
  await null;
  probed = true;
}
