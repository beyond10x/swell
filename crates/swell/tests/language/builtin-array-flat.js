// sec-array.prototype.flat: normal "1,2,3,4|3|1,2,3|4"
// flat (sec-flattenintoarray) flattens every level with Infinity and one by default, skipping
// holes; flatMap (sec-array.prototype.flatmap) flattens what its mapper answers one level only.
var nested = [1, [2, [3, [4]]]];
[nested.flat(Infinity).join(), nested.flat().length, [1, , [2, , 3]].flat().join(),
  [1, 2].flatMap(function (v) { return [v, [v]]; }).length].join("|");
