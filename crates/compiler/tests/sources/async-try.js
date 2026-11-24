// sec-await: an await inside try, catch and finally blocks, and a return of an await that goes through a finally block
async function t() {
  try {
    await 1;
  } catch (e) {
    await e;
  } finally {
    await 2;
  }
  try {
    return await 3;
  } finally {
    await 4;
  }
}
