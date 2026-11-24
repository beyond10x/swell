// sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset: a for await, whose iterator is got with kind async, each result of next awaited before its check, and the call of return awaited on the way out
async function l(list) {
  for await (const x of list) {
    if (x) break;
  }
}
