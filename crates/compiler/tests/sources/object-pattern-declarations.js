// sec-runtime-semantics-keyedbindinginitialization: a var pattern resolves each name before it reads the property, a let pattern initializes its bindings, a default runs only for undefined
function f(o) { var { a, b: c = 1 } = o; let { d, e: { g } } = o; return a + c + d + g; }
