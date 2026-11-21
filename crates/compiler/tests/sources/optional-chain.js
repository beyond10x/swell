// sec-optional-chaining-chain-evaluation: a nullish base short-circuits the whole chain to undefined; a call keeps its this; delete of a short-circuited chain is true
a?.b.c;
o.m?.();
delete a?.b;
