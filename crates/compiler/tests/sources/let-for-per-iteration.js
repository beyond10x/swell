// sec-createperiterationenvironment: a for (let ...) whose binding a closure captures copies its record before the first test and after each body
var fs = [];
for (let i = 0; i < 2; i++) { fs[i] = function () { return i; }; }
