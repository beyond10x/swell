// sec-function-calls: normal 610
function fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
fib(15);
