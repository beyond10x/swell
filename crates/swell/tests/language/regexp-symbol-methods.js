// sec-regexp.prototype-%symbol.replace%: normal "2024-11-03|a,b|b-c-d|2|a,1,b,2,c|-1"
// The four methods of RegExp.prototype keyed by well-known symbols, called through the String
// methods that hand a RegExp to them: replace with captures and with a function, match with
// and without g, split with a capture, and search.
var date = "03.11.2024".replace(/(\d+)\.(\d+)\.(\d+)/, "$3-$2-$1");
var all = "a1b2".match(/[a-z]/g).join();
var dashed = "b c d".replace(/\s/g, function (space, position) { return "-"; });
var first = "x1y22z333".match(/\d+(?=z)/)[0].length;
var parts = "a1b2c".split(/(\d)/).join();
var missing = "abc".search(/\d/);
[date, all, dashed, first, parts, missing].join("|");
