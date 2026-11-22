// sec-arrow-function-definitions: an arrow in an arrow reads the binding of the function around both through two arrows that make no environment, and this through its own frame
function f(a) { return () => () => a + this; }
