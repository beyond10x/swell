// sec-meta-properties: parses
function f() { return new.target; }
var g = function () { return new   .   target; };
var h = function (a = new.target) { return a; };
function k() { return () => () => new.target; }
function m() { return (a = new.target) => a; }
function* n() { yield new.target; }
async function p() { return await new.target; }
async function* q() { yield new.target; }
var o = { m() { return new.target; }, get x() { return new.target; }, set x(v) { new.target; } };
function r() { new new.target(); new.target(); new.target.name; new.target`x`; }
function s() { void new.target; typeof new.target; delete (new.target); }
function t() { return eval("new.target"); }
