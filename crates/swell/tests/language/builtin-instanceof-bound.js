// sec-ordinaryhasinstance: normal "truefalse"
// Step 2: a bound function answers for its target.
function F() {}
var B = F.bind(null);
"" + (new F() instanceof B) + ({} instanceof B);
