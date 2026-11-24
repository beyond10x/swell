// sec-await: an await inside a for-of, whose iterator stays on the operand stack while the frame is suspended
async function l(list) {
  for (var x of list) {
    await x;
  }
}
