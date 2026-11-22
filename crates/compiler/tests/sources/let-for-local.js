// sec-forbodyevaluation: a for (let ...) binding no closure captures is a local slot, copied by nothing, checked at each use
for (let i = 0; i < 2; i++) { i; }
