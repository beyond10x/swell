// sec-left-hand-side-expressions-static-semantics-early-errors: SyntaxError "template literal after an optional chain"
// A template after a call in an optional chain is OptionalChain TemplateLiteral, not a tagged
// template.
a?.b()`c`;
