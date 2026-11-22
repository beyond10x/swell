// sec-runtime-semantics-catchclauseevaluation: a catch pattern and a for-in pattern destructure the exception and each key into names of their own scopes
function f(o) { try { g(); } catch ({ message }) { return message; } for (let { length } in o) { h(length); } }
