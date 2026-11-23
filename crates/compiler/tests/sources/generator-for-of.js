// sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset: a yield inside a for-of, whose return closes the iterator, and one inside an array pattern
function* f(list) {
  for (var x of list) {
    yield x;
  }
  var [y = yield] = list;
}
