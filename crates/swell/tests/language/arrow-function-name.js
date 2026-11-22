// sec-runtime-semantics-namedevaluation: normal true
// An anonymous arrow takes the name of the binding or key it is assigned to, and has no prototype.
var named = () => 0;
var o = { key: (x) => x };
var assigned;
assigned = x => x;
var anonymous = [() => 0][0];
named.name === "named" && o.key.name === "key" && assigned.name === "assigned"
  && anonymous.name === "" && !named.hasOwnProperty("prototype");
