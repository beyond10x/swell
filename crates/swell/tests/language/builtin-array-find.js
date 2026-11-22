// sec-array.prototype.find: normal "3|2|4|3|-1"
// find, findIndex, findLast and findLastIndex (sec-findviapredicate) answer the first element, or
// its index, that the predicate selects, ascending for the first two and descending for the last
// two; none selected is -1 for an index.
var a = [1, 2, 3, 4];
function big(v) { return v > 2; }
[a.find(big), a.findIndex(big), a.findLast(big), a.findLastIndex(big),
  a.findIndex(function () { return false; })].join("|");
