// sec-gettemplateobject: normal true
// One template object per site: the same on each evaluation of a site, from a loop and from a
// function called again; another one for a second site of the same text; frozen, with `raw`.
function tag(s) { return s; }
function site() { return tag`a${1}b`; }
var objects = [];
for (var i = 0; i < 3; i++) objects.push(site());
var other = tag`a${1}b`;
objects[0] === objects[1] && objects[1] === objects[2] && other !== objects[0]
  && Object.isFrozen(objects[0]) && Object.isFrozen(objects[0].raw)
  && objects[0].length === 2 && objects[0][1] === "b" && objects[0].raw[0] === "a";
