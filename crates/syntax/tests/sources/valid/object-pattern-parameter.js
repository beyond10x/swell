// sec-function-definitions: parses
function f({ a, b = a }, { c: { d } } = {}) {}
var g = function ({ e }) {};
var h = ({ i, j = 1 }, { k } = {}) => i + j + k;
var l = ({}) => 0;
var o = { m({ n }) {}, set s({ t }) {} };
